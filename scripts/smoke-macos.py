#!/usr/bin/env python3
"""Run against a real logged-in pasteboard; snapshot and restore its raw items."""
import argparse
import json
import os
from pathlib import Path
import signal
import subprocess
import tarfile
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument('--bin-dir', type=Path, default=ROOT / 'target/debug')
args = parser.parse_args()
binaries = args.bin_dir.resolve()

def run(argv, **kwargs):
    return subprocess.run([str(x) for x in argv], check=True, capture_output=True, **kwargs)

with tempfile.TemporaryDirectory(prefix='rf-smoke-') as folder:
    root = Path(folder).resolve()
    helper = root / 'pasteboard'
    run(['swiftc', ROOT / 'scripts/pasteboard.swift', '-o', helper])
    snapshot = root / 'before.plist'
    run([helper, 'snapshot', snapshot])
    env = dict(os.environ, RF_DATA_DIR=str(root / 'data'))
    src = root / "中文 space ' $ # %\n目录"
    src.mkdir()
    file = src / "中文 #%.txt"
    file.write_text('rf smoke 世界\n')
    (src / 'empty').mkdir()
    (src / 'link').symlink_to(file.name)
    rf = binaries / 'rf'
    rff = binaries / 'rff'
    checks = []
    def passed(name):
        checks.append(name)
        print('PASS', name, flush=True)
    def invoke(*argv, cwd=root):
        return run([rf, *argv], cwd=cwd, env=env).stdout
    def files():
        return json.loads(run([helper, 'files']).stdout)
    try:
        assert invoke(cwd=src) == (str(src) + '\n').encode()
        assert run([helper, 'text']).stdout == str(src).encode()
        passed('no arguments: cwd text, exact stdout, no added clipboard newline')
        assert invoke('--', file) == (str(file) + '\n').encode()
        assert run([helper, 'text']).stdout == str(file).encode()
        passed('Unicode, whitespace, apostrophe, dollar, newline, hash and percent')
        alias = root / 'alias'
        alias.symlink_to(file)
        assert invoke(alias) == (str(file) + '\n').encode()
        passed('top-level symlink resolves')
        hyphen = root / '-f'
        hyphen.write_text('x')
        assert invoke('--', '-f') == (str(hyphen) + '\n').encode()
        passed('option terminator preserves dash-prefixed paths')
        assert invoke('-f', file) == (str(file) + '\n').encode()
        assert files() == [str(file)]
        assert files() == [str(file)]
        assert b'public.file-url' in run([helper, 'types']).stdout
        assert file.read_text() == 'rf smoke 世界\n'
        passed('native file URL, repeated read after process exit, original retained')
        assert run([rff, file], env=env).stdout == (str(file) + '\n').encode()
        assert files() == [str(file)]
        passed('rff matches rf -f')
        archive = Path(invoke('-f', src).decode().removesuffix('\n'))
        assert archive.is_file() and files() == [str(archive)]
        with tarfile.open(archive) as tar:
            contents = tar.extractfile(src.name + '/' + file.name).read()
            assert contents == file.read_bytes()
            assert tar.getmember(src.name + '/link').issym()
            assert tar.getmember(src.name + '/empty').isdir()
        passed('directory tar contents, empty directory and preserved symlink')
        before = files()
        missing = subprocess.run([rf, root / 'missing'], env=env, capture_output=True)
        assert missing.returncode == 1 and not missing.stdout
        assert files() == before
        passed('failure keeps clipboard and emits no success path')
        processes = [subprocess.Popen([rf, '-f', src], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE) for _ in range(2)]
        results = [p.communicate(timeout=20) for p in processes]
        assert all(p.returncode == 0 for p in processes), results
        assert results[0][0] != results[1][0]
        passed('concurrent archives use unique paths')
        large = root / 'large'
        large.mkdir()
        with (large / 'sparse').open('wb') as stream:
            stream.truncate(2 * 1024**3)
        baseline = set((root / 'data/archives').glob('rf-job-*'))
        old = files()
        for command in ([rf, '-f', large], [rff, large]):
            refused = subprocess.run(command, env=env, capture_output=True)
            assert refused.returncode == 1 and not refused.stdout
            assert b'--allow-large' in refused.stderr
            assert files() == old
            assert set((root / 'data/archives').glob('rf-job-*')) == baseline
        passed('large directory refused before archiving; clipboard unchanged')
        process = subprocess.Popen([rf, '-f', '--allow-large', large], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        deadline = time.monotonic() + 10
        while not list((root / 'data/archives').glob('rf-job-*/archive.part')):
            assert process.poll() is None and time.monotonic() < deadline
            time.sleep(0.01)
        process.send_signal(signal.SIGINT)
        stdout, stderr = process.communicate(timeout=10)
        assert process.returncode == 130, (process.returncode, stderr)
        assert not stdout and files() == old
        assert set((root / 'data/archives').glob('rf-job-*')) == baseline
        passed('SIGINT during large archive: exit 130, no residue, clipboard unchanged')
        preview = invoke('--clean-archives')
        assert preview and archive.exists()
        invoke('--clean-archives', '--yes')
        assert not archive.exists() and file.exists()
        passed('cleanup previews then deletes archives only')
        print(json.dumps({'passed': len(checks), 'finder_paste': 'manual; automation permission unavailable'}, ensure_ascii=False))
    finally:
        run([helper, 'restore', snapshot])
