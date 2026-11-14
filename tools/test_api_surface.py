"""External export boundaries, independent of source-module layout."""
import unittest
from api_surface import inventory


class Exports(unittest.TestCase):
    def test_reexport_exposes_only_public_members_of_private_module(self):
        def item(name, inner, visibility='public'):
            return {'name': name, 'crate_id': 0, 'visibility': visibility, 'inner': inner}
        data = {'includes_private': True, 'format_version': 61, 'root': 0, 'index': {
            '0': item('sample', {'module': {'items': [1, 3]}}),
            '1': item('internal', {'module': {'items': [2]}}, 'crate'),
            '2': item('Store', {'struct': {'kind': {'plain': {'fields': [4, 5]}}, 'impls': [6]}}),
            '3': item(None, {'use': {'id': 2, 'name': 'Arena', 'is_glob': False}}),
            '4': item('version', {'struct_field': {} }),
            '5': item('raw', {'struct_field': {} }, 'crate'),
            '6': item(None, {'impl': {'trait': None, 'items': [7, 8]}}),
            '7': item('new', {'function': {}}),
            '8': item('raw_write', {'function': {}}, 'crate'),
        }}
        self.assertEqual(inventory(data)['entries'], [
            'function sample::Arena::new', 'module sample',
            'struct sample::Arena', 'struct_field sample::Arena::version'])

    def test_trait_members_and_enum_payloads_are_public_without_pub_keywords(self):
        data = {'includes_private': True, 'format_version': 61, 'root': 0, 'index': {}}
        for key, name, visibility, inner in [
            (0, 'sample', 'public', {'module': {'items': [1, 3]}}),
            (1, 'Read', 'public', {'trait': {'items': [2]}}),
            (2, 'read', 'default', {'function': {}}),
            (3, 'Value', 'public', {'enum': {'variants': [4], 'impls': []}}),
            (4, 'Named', 'default', {'variant': {'kind': {'struct': {'fields': [5]}}}}),
            (5, 'name', 'default', {'struct_field': {}}),
        ]:
            data['index'][str(key)] = {'crate_id': 0, 'name': name,
                                     'visibility': visibility, 'inner': inner}
        result = inventory(data)['entries']
        self.assertIn('function sample::Read::read', result)
        self.assertIn('struct_field sample::Value::Named::name', result)

    def test_unknown_rustdoc_format_requires_review(self):
        with self.assertRaisesRegex(ValueError, 'review format changes'):
            inventory({'format_version': 999})

    def test_incomplete_json_requires_explicit_inventory_flags(self):
        with self.assertRaisesRegex(ValueError, 'document-private-items'):
            inventory({'format_version': 61, 'includes_private': False})

    def test_external_glob_is_explicit_not_silently_dropped(self):
        data = {'includes_private': True, 'format_version': 61, 'root': 0, 'index': {
            '0': {'name': 'sample', 'crate_id': 0, 'visibility': 'public',
                  'inner': {'module': {'items': [1]}}},
            '1': {'name': None, 'crate_id': 0, 'visibility': 'public',
                  'inner': {'use': {'id': 9, 'name': '*', 'is_glob': True,
                                    'source': 'dependency::compiler'}}},
        }}
        self.assertIn('external_glob sample::* => dependency::compiler',
                      inventory(data)['entries'])


if __name__ == '__main__':
    unittest.main()
