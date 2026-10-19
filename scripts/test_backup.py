import json
from pathlib import Path
import sqlite3
import tempfile
import unittest
from backup import backup, restore, verify

class BackupTest(unittest.TestCase):
    def test_wal_snapshot_restore_and_tamper_detection(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp); data=root/'data';data.mkdir()
            db=sqlite3.connect(data/'osint.db');db.execute('PRAGMA journal_mode=WAL')
            db.executescript('CREATE TABLE reports(id);INSERT INTO reports VALUES(7);CREATE TABLE auth_sessions(hash);INSERT INTO auth_sessions VALUES("old");')
            (data/'evidence').mkdir();(data/'evidence'/'source.pdf').write_bytes(b'test document')
            (data/'ai-settings.json').write_text('{"test":"private"}')
            self.assertTrue(backup(data,root/'snapshot')['verified'])
            self.assertTrue(restore(root/'snapshot',root/'restored')['sessions_revoked'])
            with sqlite3.connect(root/'restored'/'osint.db') as restored:
                self.assertEqual(restored.execute('SELECT id FROM reports').fetchone()[0],7)
                self.assertEqual(restored.execute('SELECT count(*) FROM auth_sessions').fetchone()[0],0)
            with self.assertRaises(FileExistsError): restore(root/'snapshot',root/'restored')
            (root/'snapshot'/'evidence'/'source.pdf').write_bytes(b'changed')
            with self.assertRaises(ValueError):verify(root/'snapshot')
            db.close()
    def test_manifest_cannot_escape_root(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);(root/'manifest.json').write_text(json.dumps({'version':1,'files':{'../osint.db':'bad','osint.db':'bad'}}))
            with self.assertRaises(ValueError):verify(root)

if __name__=='__main__':unittest.main()
