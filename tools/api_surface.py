"""Inventory named Rust exports from pinned rustdoc JSON, including re-exports.

This is an export-growth guard, not a signature/ABI/semver compatibility checker.
It includes public fields, enum variants, trait members and inherent methods.
It excludes trait implementations, Deref/alias-inherited members and signatures.
Generate the input with the repository's pinned rustdoc; no compiler is invoked
and no baseline is changed implicitly by this tool.
"""
import argparse
import json
from pathlib import Path


def inventory(document, on_item=None):
    if document['format_version'] != 61:
        raise ValueError('Expected pinned rustdoc JSON format 61; review format changes first')
    if not document.get('includes_private'):
        raise ValueError('Generate JSON with --document-private-items --document-hidden-items')
    index = document['index']
    root = index[str(document['root'])]
    entries = set()

    def visit(identity, path, ancestors=(), exported=False):
        if identity is None or identity in ancestors:
            return
        item = index.get(str(identity))
        if item is None or item['crate_id'] != root['crate_id']:
            # The named re-export is ours; its external implementation is not.
            entries.add('external ' + path)
            return
        if not exported and item['visibility'] != 'public':
            return
        kind, body = next(iter(item['inner'].items()))
        ancestors = (*ancestors, identity)
        if kind == 'use':
            if body['is_glob']:
                target = index.get(str(body['id']))
                if target is None or target['crate_id'] != root['crate_id']:
                    entries.add('external_glob ' + path.rsplit('::', 1)[0] + '::* => ' + body['source'])
                    return
                if 'module' not in target['inner']:
                    raise ValueError('Unsupported non-module glob: ' + path)
                children(target['inner']['module']['items'], path.rsplit('::', 1)[0], ancestors)
            else:
                visit(body['id'], path.rsplit('::', 1)[0] + '::' + body['name'], ancestors, True)
            return
        entries.add(kind + ' ' + path)
        if on_item is not None:
            on_item(kind + ' ' + path, item)
        if kind == 'module':
            children(body['items'], path, ancestors)
        elif kind == 'trait':
            children(body['items'], path, ancestors, True)
        elif kind == 'enum':
            children(body['variants'], path, ancestors, True)
        elif kind in ('struct', 'union', 'variant'):
            shape = body.get('kind', {})
            fields = body.get('fields', [])
            if isinstance(shape, dict):
                if 'plain' in shape:
                    fields = shape['plain']['fields']
                elif 'struct' in shape:
                    fields = shape['struct']['fields']
                elif 'tuple' in shape:
                    fields = shape['tuple']
            children(fields, path, ancestors, kind == 'variant')
        if isinstance(body, dict):
            for implementation in body.get('impls', []):
                impl = index[str(implementation)]['inner']['impl']
                if impl['trait'] is None:
                    children(impl['items'], path, ancestors)

    def children(identities, parent, ancestors, exported=False):
        for identity in identities:
            if identity is None:
                continue
            item = index.get(str(identity))
            if item is None:
                raise ValueError('Missing local item ' + str(identity))
            name = item['name']
            if name is None and 'use' in item['inner']:
                name = item['inner']['use']['name']
            if name is None:
                continue
            visit(identity, parent + '::' + name, ancestors, exported)

    visit(document['root'], root['name'])
    return {'schema': 1, 'rustdoc_format': 61, 'crate': root['name'],
            'scope': 'named exports; not signatures, trait implementations or ABI',
            'entries': sorted(entries)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('rustdoc', type=Path)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--write', type=Path, help='Explicitly write a reviewed inventory')
    mode.add_argument('--check', type=Path, help='Fail on additions or removals')
    args = parser.parse_args()
    result = inventory(json.loads(args.rustdoc.read_text(encoding='utf-8')))
    if args.write:
        args.write.write_bytes((json.dumps(result, indent=2) + '\n').encode())
    else:
        expected = json.loads(args.check.read_text(encoding='utf-8'))
        if expected != result:
            old, new = set(expected['entries']), set(result['entries'])
            for sign, items in [('-', old - new), ('+', new - old)]:
                for item in sorted(items):
                    print(sign, item)
            raise SystemExit('Public export inventory changed; review before updating the baseline')
    print(result['crate'], len(result['entries']), 'named exports')


if __name__ == '__main__':
    main()
