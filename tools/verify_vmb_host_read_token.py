#!/usr/bin/env python3
"""Compare host-token allocations with a retained compiler; no full-book claim."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time


def sha(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def row(path):
    return {'path': str(path), 'byte_length': path.stat().st_size, 'sha256': sha(path)}


def write(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--compiler-manifest', type=Path, required=True)
    parser.add_argument('--epoch', choices=['parent', 'current'], required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    output = args.output.resolve()
    if output.exists() or any(output.is_relative_to(root / p) for p in ['tools', 'workspace/crates', 'workspace/vendor']):
        parser.error('output must be a new evidence directory outside product sources')
    output.mkdir(parents=True)
    manifest = json.loads(args.compiler_manifest.read_text())
    dependencies = manifest.get('files', manifest)
    assert {'typaxis_core', 'typaxis_host_admission', 'serde_json'} <= dependencies.keys()
    for value in dependencies.values():
        path = Path(value['path'])
        assert path.stat().st_size == value['byte_length'] and sha(path) == value['sha256'], path
    source = root / 'tools/vmb_host_read_token_probe.rs'
    write(output / 'inputs.json', {'compiler_manifest': row(args.compiler_manifest.resolve()),
                                  'compiler_dependencies': dependencies, 'source': row(source)})
    binary = output / 'probe'
    directories = sorted({str(Path(v['path']).parent) for v in dependencies.values()})
    argv = ['rustc', '--edition=2021', '-C', 'debuginfo=0', '-C', 'overflow-checks=on']
    for directory in directories:
        argv += ['-L', 'dependency=' + directory]
    for name, value in sorted(dependencies.items()):
        argv += ['--extern', name + '=' + value['path']]
    if args.epoch == 'current':
        argv += ['--cfg', 'token_current']
    argv += [str(source), '-o', str(binary)]
    commands = []
    for name, command in [('compile', argv), ('run', [str(binary), str(output / 'host'), args.epoch])]:
        started = time.time()
        record = {'argv': command, 'cwd': str(root), 'state': 'running', 'started_unix': started}
        write(output / (name + '.command.json'), record)
        with (output / (name + '.stdout')).open('xb') as out, (output / (name + '.stderr')).open('xb') as err:
            child = subprocess.run(command, cwd=root, stdout=out, stderr=err, timeout=50, check=False)
        record.update(state='complete', exit_code=child.returncode, elapsed_seconds=time.time() - started)
        write(output / (name + '.command.json'), record)
        commands.append(record)
        assert child.returncode == 0, (output / (name + '.stderr')).read_text()
    facts = json.loads((output / 'run.stdout').read_text())
    assert facts['passed'] and facts['epoch'] == args.epoch
    write(output / 'report.json', {'passed': True, 'scope': 'generic_host_read_token',
        'full_book_acceptance': False, 'public_profile_acceptance': False,
        'epoch': args.epoch, 'facts': facts, 'binary': row(binary), 'commands': commands})
    print(json.dumps({'passed': True, 'epoch': args.epoch, 'facts': facts}, ensure_ascii=False))


if __name__ == '__main__':
    main()
