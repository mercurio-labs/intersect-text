"""Review guard for Rust signatures and Serde type declarations, not semver proof.

Requires pinned rustdoc JSON with private and hidden items. Hashes exclude rustdoc
IDs, source locations and documentation. --describe prints a current declaration.
Function/codec bodies, blanket/auto traits, ABI and transitive external definitions
are outside this guard; serialized acceptance still requires executable controls.
The build-generated SOURCE_IDENTITY constant retains its type contract but not
its value: the latter intentionally changes with implementation source.
"""
import argparse
import hashlib
import json
from pathlib import Path

from api_surface import inventory


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(',', ':'))


class Contracts:
    def __init__(self, document):
        self.index = document['index']
        self.root = self.index[str(document['root'])]
        self.paths = {k: '::'.join(v['path']) for k, v in document.get('paths', {}).items()}
        self.owners = {}
        self.trait_members = set()
        self.serde = {}

        def names(identity, parent):
            item = self.index[str(identity)]
            kind, body = next(iter(item['inner'].items()))
            path = parent + '::' + item['name'] if parent else item['name']
            if path:
                self.paths.setdefault(str(identity), path)
            if kind == 'module':
                for child in body['items']:
                    if self.index[str(child)]['name'] is not None:
                        names(child, path)
        names(document['root'], '')
        for item in self.index.values():
            if item['crate_id'] != self.root['crate_id']:
                continue
            if 'trait' in item['inner']:
                self.trait_members.update(item['inner']['trait']['items'])
            impl = item['inner'].get('impl')
            if impl is None:
                continue
            for member in impl['items']:
                self.owners[member] = impl
            trait = impl.get('trait')
            if trait and trait['path'].split('::')[-1] in ('Serialize', 'Deserialize'):
                # Resolve the trait identity: a local trait with this name is not Serde.
                full = self.resolve(trait['id'])
                if full.startswith(('serde::', 'serde_core::')):
                    target = impl['for'].get('resolved_path')
                    if target:
                        self.serde.setdefault(str(target['id']), set()).add(full.split('::')[-1])

    def resolve(self, identity):
        try:
            return self.paths[str(identity)]
        except KeyError as error:
            raise ValueError('Unresolved declaration identity; extend the pinned format reader') from error

    def normalize(self, value):
        if isinstance(value, list):
            return [self.normalize(v) for v in value]
        if not isinstance(value, dict):
            return value
        result = {}
        for key, child in value.items():
            if key == 'id':
                result['declaration'] = self.resolve(child)
            elif key != 'path' or 'id' not in value:
                result[key] = self.normalize(child)
        return result

    @staticmethod
    def attributes(item):
        # Derive capabilities are captured from explicit trait impl headers below.
        return [a for a in item.get('attrs', []) if any(
            word in encoded(a) for word in ('serde', 'repr', 'non_exhaustive'))]

    def fields(self, identities, wire=False, variant=False):
        result = []
        for identity in identities:
            if identity is None:
                result.append({'private': True})
                continue
            field = self.index[str(identity)]
            if not wire and not variant and field['visibility'] != 'public':
                result.append({'private': True})
            else:
                result.append({'name': field['name'], 'type': self.normalize(field['inner']),
                               'attrs': self.attributes(field)})
        return result

    def implementation(self, impl):
        return self.normalize({k: impl[k] for k in
                               ('trait', 'for', 'generics', 'is_unsafe', 'is_negative') if k in impl})

    def declaration(self, item, wire=False):
        kind, body = next(iter(item['inner'].items()))
        result = {'kind': kind, 'attrs': self.attributes(item)}
        if item.get('deprecation') is not None:
            result['deprecation'] = item['deprecation']
        if kind in ('struct', 'union', 'enum', 'trait', 'variant'):
            result['definition'] = self.normalize({k: v for k, v in body.items() if k not in
                                                  ('kind', 'fields', 'variants', 'items', 'impls', 'implementations')})
            if kind in ('struct', 'variant'):
                shape = body['kind']
                if isinstance(shape, str):
                    result['shape'] = shape
                else:
                    shape_name, fields = next(iter(shape.items()))
                    result['shape'] = shape_name
                    result['fields'] = self.fields(fields if isinstance(fields, list) else fields['fields'], wire, kind == 'variant')
            if kind == 'union':
                result['fields'] = self.fields(body['fields'], wire)
            if kind == 'enum':
                result['variants'] = [{'name': self.index[str(v)]['name'],
                                       'definition': self.declaration(self.index[str(v)], wire)} for v in body['variants']]
            implementations = []
            for identity in body.get('impls', []):
                impl = self.index[str(identity)]['inner']['impl']
                if impl['trait'] and not impl.get('is_synthetic') and impl.get('blanket_impl') is None:
                    contract = self.implementation(impl)
                    # Associated types/consts affect callers; method bodies do not.
                    contract['associated'] = [{
                        'name': self.index[str(m)]['name'],
                        'definition': self.declaration(self.index[str(m)])
                    } for m in impl['items'] if 'function' not in self.index[str(m)]['inner']]
                    implementations.append(contract)
            result['explicit_traits'] = sorted(implementations, key=encoded)
        elif kind == 'function':
            result['definition'] = self.normalize({k: v for k, v in body.items() if k not in ('has_body', 'default_unstable')})
            if item['id'] in self.trait_members:
                result['provided'] = body.get('has_body')
            if item['id'] in self.owners:
                result['impl_context'] = self.implementation(self.owners[item['id']])
        else:
            result['definition'] = self.normalize(body)
            if kind == 'constant' and self.resolve(item['id']) == self.root['name'] + '::SOURCE_IDENTITY':
                result['definition']['const'] = '<build-generated source identity>'
        return result


def contracts(document):
    # Validate format before traversing compiler-specific structures.
    exported = {}
    inventory(document, lambda key, item: exported.setdefault(key, item))
    reader = Contracts(document)
    declarations = {key: reader.declaration(item) for key, item in exported.items()
                    if 'module' not in item['inner']}
    for identity, capabilities in reader.serde.items():
        item = reader.index.get(identity)
        if item is None or item['crate_id'] != reader.root['crate_id']:
            continue
        if next(iter(item['inner'])) not in ('struct', 'enum', 'union'):
            continue
        declarations['serde ' + reader.resolve(identity)] = {
            'capabilities': sorted(capabilities), 'declaration': reader.declaration(item, wire=True)}
    return reader.root['name'], declarations


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('rustdoc', type=Path)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument('--write', type=Path)
    mode.add_argument('--check', type=Path)
    mode.add_argument('--describe', help='Exact contract key, e.g. serde intersect_model::rcore::SavedModel')
    args = parser.parse_args()
    crate, declarations = contracts(json.loads(args.rustdoc.read_text(encoding='utf-8')))
    if args.describe:
        print(json.dumps(declarations[args.describe], indent=2, sort_keys=True))
        return
    result = {'schema': 1, 'rustdoc_format': 61, 'crate': crate,
              'scope': 'signatures, explicit trait impl headers, Serde declarations; not codec bodies or semver proof',
              'contracts': {k: hashlib.sha256(encoded(v).encode()).hexdigest() for k, v in sorted(declarations.items())}}
    if args.write:
        args.write.write_bytes((json.dumps(result, indent=2)+'\n').encode())
    else:
        expected = json.loads(args.check.read_text(encoding='utf-8'))
        if expected != result:
            old, new = expected['contracts'], result['contracts']
            for key in sorted(old.keys() | new.keys()):
                if old.get(key) != new.get(key):
                    print('changed' if key in old and key in new else ('added' if key in new else 'removed'), key)
            raise SystemExit('Contract changed; review declarations and runtime controls before updating')
    print(crate, len(declarations), 'signature/Serde contracts')


if __name__ == '__main__':
    main()
