"""Verify reviewed notice references against an immutable Git tree.

The policy describes manually reviewed content families. This check does not
infer licenses from filenames or grant compatibility to unknown new inputs.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def git(root, *args):
    return subprocess.check_output(['git', *args], cwd=root)


def check(policy, files, read):
    components = []
    for component in policy['components']:
        matched = sorted(p for p in files if any(
            p.startswith(prefix) if prefix.endswith('/') else p == prefix
            for prefix in component['inputs']))
        if not matched:
            raise ValueError('No inputs for notice family: ' + component['id'])
        notices = []
        for row in component['notices']:
            path = row['path']
            if path not in files:
                raise ValueError('Missing notice: ' + path)
            if hashlib.sha256(read(path)).hexdigest() != row['sha256']:
                raise ValueError('Notice changed; review required: ' + path)
            notices.append(row)
        if not notices:
            raise ValueError('No notices for: ' + component['id'])
        components.append({'id': component['id'], 'scope': component['scope'],
                           'input_files': len(matched), 'notices': notices})
    return components


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--revision', default='HEAD')
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    revision = git(root, 'rev-parse', '--verify', args.revision + '^{commit}').decode().strip()
    files = {}
    for row in git(root, 'ls-tree', '-r', '-l', '-z', revision).split(b'\0'):
        if not row:
            continue
        metadata, name = row.split(b'\t', 1)
        mode, kind, oid, size = metadata.decode().split()
        if kind != 'blob':
            raise ValueError('Review non-blob Git entry: ' + name.decode())
        files[name.decode()] = {'git_blob': oid, 'bytes': int(size), 'mode': mode}
    def read(path):
        return git(root, 'cat-file', 'blob', files[path]['git_blob'])
    policy = json.loads(read('docs/repository-notices.json'))
    components = check(policy, files, read)
    result = {'revision': revision, 'scope': policy['scope'],
              'policy_sha256': hashlib.sha256(read('docs/repository-notices.json')).hexdigest(),
              'file_count': len(files), 'bytes': sum(r['bytes'] for r in files.values()),
              'components': components, 'files': files}
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'revision': revision, 'files': len(files),
                      'notice_families': len(components), 'output': str(args.out)}))


if __name__ == '__main__':
    main()
