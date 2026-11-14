"""Portable recorded-source authority and complete native observation controls."""
import importlib.util,json,pathlib,unittest,os,sys
sys.dont_write_bytecode=True
from unittest import mock
ROOT=pathlib.Path(os.environ.get('INTERSECT_TEST_CORPUS',pathlib.Path(__file__).resolve().parents[1]/'fixtures/xtext/contextual-first-rule'))
spec=importlib.util.spec_from_file_location('contextual_entry_evidence',ROOT/'verify.py')
corpus=importlib.util.module_from_spec(spec);spec.loader.exec_module(corpus)
class NativeContextualEvidence(unittest.TestCase):
    def test_complete_before_after_and_actual_full_model_overlap(self):
        result=corpus.verify_native()
        self.assertEqual((result['graphs'],result['planned_inputs'],result['candidate_complete_requests']),(20,128,394))
        self.assertEqual((result['actual_upstream_resource_overlap'],result['actual_accepted_full_object_models']),(29,7))
        self.assertFalse(result['native_ecore_award'])
    def test_historical_absolute_paths_cannot_become_owned_fixture_inputs(self):
        for value in ['../outside','D:/historical/target/model.ecore','native\\wrong.json','/absolute']:
            with self.subTest(value=value),self.assertRaisesRegex(AssertionError,'Unsafe portable path'):corpus.owned(value)
    def test_portable_metadata_cannot_substitute_a_new_compilation_recipe(self):
        original=corpus.load
        def forged(name):
            value=original(name)
            if name=='native/native-cases.json':value['cases'][0]['compilation']['namespace']='urn:forged'
            return value
        with mock.patch.object(corpus,'load',side_effect=forged),self.assertRaisesRegex(AssertionError,'Metadata-only adaptation'):
            corpus.verify_native()
if __name__=='__main__':unittest.main()
