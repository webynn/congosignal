#!/usr/bin/env python3
"""Private local SQLite/archive snapshots. Restore only into a new directory.
Backups may contain provider credentials and private notes. Protect off-host copies.
"""
from contextlib import closing
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import sqlite3
from datetime import datetime, timezone


def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b''):
            h.update(chunk)
    return h.hexdigest()


def private_copy(source, target):
    if source.is_symlink():
        raise ValueError('Symlinks are not included in snapshots')
    target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    shutil.copyfile(source, target)
    target.chmod(0o600)


def check_db(path):
    with closing(sqlite3.connect(f'{path.as_uri()}?mode=ro', uri=True)) as db:
        if db.execute('PRAGMA integrity_check').fetchone()[0] != 'ok':
            raise ValueError('SQLite integrity check failed')


def backup(data, output):
    data, output = Path(data).resolve(), Path(output).absolute()
    if not (data / 'osint.db').is_file():
        raise ValueError('Expected osint.db in the data directory')
    output.mkdir(parents=True, exist_ok=False, mode=0o700)
    # SQLite online backup captures committed WAL transactions without stopping the worker.
    with closing(sqlite3.connect(f'{(data / "osint.db").as_uri()}?mode=ro', uri=True)) as src:
        with closing(sqlite3.connect(output / 'osint.db')) as dst:
            src.backup(dst, pages=256, sleep=0.05)
            dst.execute("PRAGMA journal_mode=DELETE")
    (output / 'osint.db').chmod(0o600)
    check_db(output / 'osint.db')
    for directory in ('evidence', 'agreements'):
        root = data / directory
        if root.is_symlink():
            raise ValueError('Archive directory must not be a symlink')
        if root.exists():
            for source in root.rglob('*'):
                if source.is_symlink():
                    raise ValueError('Archive symlink rejected')
                if source.is_file():
                    private_copy(source, output / directory / source.relative_to(root))
    settings = data / 'ai-settings.json'
    if settings.exists():
        private_copy(settings, output / settings.name)
    files = {str(p.relative_to(output)): sha(p) for p in sorted(output.rglob('*')) if p.is_file()}
    manifest = dict(version=1, created_at=datetime.now(timezone.utc).isoformat(), files=files,
                    note='Contains secrets if AI settings were present. Environment variables/.env and rebuildable raw mineral downloads are not included. Use matching application release.')
    (output / 'manifest.json').write_text(json.dumps(manifest, indent=2))
    (output / 'manifest.json').chmod(0o600)
    return verify(output)


def verify(snapshot):
    root = Path(snapshot).resolve()
    if (root / 'manifest.json').is_symlink():
        raise ValueError('Manifest symlink rejected')
    manifest = json.loads((root / 'manifest.json').read_text())
    if manifest.get('version') != 1 or 'osint.db' not in manifest.get('files', {}):
        raise ValueError('Unknown or incomplete snapshot')
    for name, expected in manifest['files'].items():
        path = root / name
        if Path(name).is_absolute() or '..' in Path(name).parts or any((root / Path(*Path(name).parts[:i])).is_symlink() for i in range(1, len(Path(name).parts) + 1)):
            raise ValueError('Unsafe snapshot path')
        if not path.is_file() or sha(path) != expected:
            raise ValueError(f'Snapshot checksum mismatch: {name}')
    check_db(root / 'osint.db')
    return dict(verified=True, files=len(manifest['files']), created_at=manifest['created_at'])


def restore(snapshot, destination):
    verify(snapshot)
    root, target = Path(snapshot).resolve(), Path(destination).absolute()
    manifest = json.loads((root / 'manifest.json').read_text())
    target.mkdir(parents=True, exist_ok=False, mode=0o700)
    for name in manifest['files']:
        private_copy(root / name, target / name)
    # Restored session tokens must not reactivate old browser sessions.
    with sqlite3.connect(target / 'osint.db') as db:
        for table in ('auth_sessions', 'auth_states'):
            if db.execute("SELECT 1 FROM sqlite_master WHERE type='table' AND name=?", (table,)).fetchone():
                db.execute(f'DELETE FROM {table}')
    check_db(target / 'osint.db')
    return dict(restored=True, files=len(manifest['files']), sessions_revoked=True)


if __name__ == '__main__':
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    p = sub.add_parser('create'); p.add_argument('--data', default='data'); p.add_argument('--output', required=True)
    p = sub.add_parser('verify'); p.add_argument('snapshot')
    p = sub.add_parser('restore'); p.add_argument('snapshot'); p.add_argument('--destination', required=True)
    a = parser.parse_args()
    try:
        result = backup(a.data, a.output) if a.command == 'create' else verify(a.snapshot) if a.command == 'verify' else restore(a.snapshot, a.destination)
        print(json.dumps(result))
    except (OSError, ValueError, sqlite3.Error) as e:
        parser.exit(1, f'Backup operation failed: {e}\n')
