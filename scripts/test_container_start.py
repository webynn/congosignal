"""Check port selection without Docker or touching /data."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
class EntrypointTest(unittest.TestCase):
    def check(self, values, expected):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)
            (p/'id').write_text('#!/bin/sh\necho 10001\n')
            (p/'probe').write_text('#!/bin/sh\nprintf "%s" "$BIND_ADDR"\n')
            for f in p.iterdir(): f.chmod(0o755)
            env={k:v for k,v in os.environ.items() if k not in ['PORT','BIND_ADDR']}
            env.update(values,PATH=d+':'+env['PATH'])
            result=subprocess.check_output(['sh',str(ROOT/'scripts/container-start.sh'),'probe'],env=env,text=True)
            self.assertEqual(result,expected)
    def test_railway_port(self): self.check({'PORT':'9123'},'0.0.0.0:9123')
    def test_explicit_bind(self): self.check({'PORT':'9123','BIND_ADDR':'127.0.0.1:8088'},'127.0.0.1:8088')
    def test_default(self): self.check({},'0.0.0.0:8080')
if __name__=='__main__': unittest.main()
