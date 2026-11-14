"""Compatibility changes must be visible without depending on rustdoc numbering."""
import copy
import unittest

from api_contracts import contracts


def fixture():
    def item(identity, name, inner, visibility='public', attrs=None):
        return {'id': identity, 'crate_id': 0, 'name': name, 'visibility': visibility,
                'attrs': attrs or [], 'inner': inner}
    return {'format_version': 61, 'includes_private': True, 'root': 0,
            'paths': {'90': {'path': ['serde', 'Serialize']}}, 'index': {
        '0': item(0, 'sample', {'module': {'items': [1, 8]}}),
        '1': item(1, 'Packet', {'struct': {'kind': {'plain': {'fields': [2, 3]}},
                                        'generics': {}, 'impls': [4, 6]}}),
        '2': item(2, 'tag', {'struct_field': {'primitive': 'u32'}}),
        '3': item(3, 'secret', {'struct_field': {'primitive': 'u8'}}, 'crate'),
        '4': item(4, None, {'impl': {'trait': None, 'for': {'generic': 'Self'},
                                    'generics': {'params': []}, 'items': [5]}}),
        '5': item(5, 'inspect', {'function': {
            'sig': {'inputs': [['self', {'borrowed_ref': {'lifetime': None,
                   'is_mutable': False, 'type': {'generic': 'Self'}}}]],
                    'output': {'primitive': 'u32'}, 'is_c_variadic': False},
            'generics': {}, 'header': {'is_unsafe': False, 'is_async': False}, 'has_body': True}}),
        '6': item(6, None, {'impl': {'trait': {'id': 90, 'path': 'Serialize', 'args': None},
            'for': {'resolved_path': {'id': 1, 'path': 'Packet', 'args': None}},
            'generics': {}, 'items': [], 'is_synthetic': False, 'blanket_impl': None}}),
        '8': item(8, 'Read', {'trait': {'items': [9], 'bounds': [], 'generics': {}}}),
        '9': item(9, 'read', {'function': {'sig': {}, 'generics': {}, 'header': {}, 'has_body': True}}, 'default'),
    }}


class Signatures(unittest.TestCase):
    def setUp(self):
        self.doc = fixture()
        self.before = contracts(self.doc)[1]

    def test_receiver_mutability_and_return_type_are_contracts(self):
        f = self.doc['index']['5']['inner']['function']
        f['sig']['inputs'][0][1]['borrowed_ref']['is_mutable'] = True
        self.assertNotEqual(self.before['function sample::Packet::inspect'],
                            contracts(self.doc)[1]['function sample::Packet::inspect'])
        self.doc = fixture()
        self.doc['index']['5']['inner']['function']['sig']['output'] = {'primitive': 'u64'}
        self.assertNotEqual(self.before, contracts(self.doc)[1])

    def test_impl_bounds_are_part_of_method_signature(self):
        self.doc['index']['4']['inner']['impl']['generics']['where_predicates'] = ['T: Clone']
        self.assertNotEqual(self.before['function sample::Packet::inspect'],
                            contracts(self.doc)[1]['function sample::Packet::inspect'])

    def test_private_serialized_field_type_is_guarded(self):
        self.doc['index']['3']['inner']['struct_field'] = {'primitive': 'u64'}
        after = contracts(self.doc)[1]
        self.assertEqual(self.before['struct sample::Packet'], after['struct sample::Packet'])
        self.assertNotEqual(self.before['serde sample::Packet'], after['serde sample::Packet'])

    def test_serde_policy_and_capability_removal_are_guarded(self):
        self.doc['index']['1']['attrs'] = [{'other': '#[serde(deny_unknown_fields)]'}]
        self.assertNotEqual(self.before['serde sample::Packet'], contracts(self.doc)[1]['serde sample::Packet'])
        self.doc = fixture()
        self.doc['index']['1']['inner']['struct']['impls'].remove(6)
        del self.doc['index']['6']
        self.assertNotIn('serde sample::Packet', contracts(self.doc)[1])
        self.assertNotEqual(self.before['struct sample::Packet'], contracts(self.doc)[1]['struct sample::Packet'])

    def test_required_trait_method_is_not_a_default_method(self):
        self.doc['index']['9']['inner']['function']['has_body'] = False
        self.assertNotEqual(self.before['function sample::Read::read'],
                            contracts(self.doc)[1]['function sample::Read::read'])

    def test_build_identity_value_is_volatile_but_its_type_is_not(self):
        self.doc['index']['0']['inner']['module']['items'].append(12)
        self.doc['index']['12'] = {'id': 12, 'crate_id': 0, 'name': 'SOURCE_IDENTITY',
            'visibility': 'public', 'inner': {'constant': {
                'type': {'primitive': 'str'}, 'const': {'expr': 'old', 'value': 'old'}}}}
        before = contracts(self.doc)[1]
        self.doc['index']['12']['inner']['constant']['const']['value'] = 'new'
        self.assertEqual(before, contracts(self.doc)[1])
        self.doc['index']['12']['inner']['constant']['type'] = {'primitive': 'u64'}
        self.assertNotEqual(before, contracts(self.doc)[1])

    def test_enum_order_and_discriminants_are_guarded(self):
        self.doc['index']['0']['inner']['module']['items'].append(20)
        self.doc['index']['20'] = {'id': 20, 'crate_id': 0, 'name': 'Mode',
            'visibility': 'public', 'inner': {'enum': {'variants': [21, 22], 'generics': {}, 'impls': []}}}
        for key, name in [(21, 'First'), (22, 'Second')]:
            self.doc['index'][str(key)] = {'id': key, 'crate_id': 0, 'name': name,
                'visibility': 'default', 'inner': {'variant': {'kind': 'plain', 'discriminant': None}}}
        before = contracts(self.doc)[1]
        self.doc['index']['20']['inner']['enum']['variants'].reverse()
        self.assertNotEqual(before['enum sample::Mode'], contracts(self.doc)[1]['enum sample::Mode'])
        self.doc['index']['20']['inner']['enum']['variants'].reverse()
        self.doc['index']['21']['inner']['variant']['discriminant'] = {'expr': '7', 'value': '7'}
        self.assertNotEqual(before['variant sample::Mode::First'], contracts(self.doc)[1]['variant sample::Mode::First'])

    def test_docs_and_compiler_item_numbering_do_not_change_contracts(self):
        self.doc['index']['1']['docs'] = 'Unrelated documentation change'
        self.doc['index']['1']['span'] = {'filename': 'another/path.rs'}
        def renumber(value, key=''):
            if isinstance(value, dict):
                return {str(int(k)+1000) if key in ('index', 'paths') else k: renumber(v, k)
                        for k, v in value.items()}
            if isinstance(value, list):
                return [renumber(v, key) for v in value]
            if type(value) is int and key not in ('format_version', 'crate_id'):
                return value+1000
            return value
        self.assertEqual(self.before, contracts(renumber(copy.deepcopy(self.doc)))[1])


if __name__ == '__main__':
    unittest.main()
