"""Refresh Text's pinned Model-owned infrastructure from an explicit checkout."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--model-checkout', type=Path, required=True)
    parser.add_argument('--revision', required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    pin_path = root / 'docs/shared-tooling.json'
    pin = json.loads(pin_path.read_text(encoding='utf-8'))
    revision = subprocess.check_output(['git', 'rev-parse', args.revision + '^{commit}'], cwd=args.model_checkout, text=True).strip()
    for name in pin['files']:
        if not name.startswith('tools/') or '..' in Path(name).parts:
            raise ValueError('Invalid infrastructure snapshot path: ' + name)
        data = subprocess.check_output(['git', 'show', revision + ':' + name], cwd=args.model_checkout)
        (root / name).write_bytes(data)
        pin['files'][name] = hashlib.sha256(data).hexdigest()
    pin['revision'] = revision
    pin_path.write_text(json.dumps(pin, indent=2) + '\n', encoding='utf-8')
    print('Updated pinned infrastructure from ' + revision + '; review the diff and run owning controls.')


if __name__ == '__main__':
    main()
