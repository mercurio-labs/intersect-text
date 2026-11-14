"""Independent complete source-generated lexical fixture checks; no JVM needed."""
from pathlib import Path
from stage_text_packages import io_path
import copy,hashlib,importlib.util,json,unittest
ROOT = io_path(Path(__file__).resolve().parent.parent)
FIXTURE=ROOT/'fixtures/xtext/full-generated-lexical'
spec=importlib.util.spec_from_file_location('full_generated_lexical',FIXTURE/'verify.py');verifier=importlib.util.module_from_spec(spec);spec.loader.exec_module(verifier)
def load(name):return json.loads((FIXTURE/name).read_text(encoding='utf-8'))
class FullGeneratedLexical(unittest.TestCase):
    def test_full_graphs_and_complete_observation_bytes_are_pinned(self):
        counts=verifier.frozen();self.assertEqual(counts['runtime_inputs'],80)
        original={c['id']:c for c in json.loads((ROOT/'fixtures/xtext/terminal-negation/cases.json').read_text(encoding='utf-8'))}
        for case in load('inputs/cases.json'):
            for source in case['sources']:
                raw=(FIXTURE/'inputs'/case['id']/source['file']).read_bytes()
                if case['id']in original:self.assertEqual(raw,original[case['id']]['sources'][0]['text'].encode('utf-8'))
                native=next(c for c in load('native-cases.json')['cases']if c['id']==case['id'])
                retained=next(s for s in native['compilation']['bundle']['sources']if s['kind']=='xtext')
                self.assertEqual(raw,retained['text'].encode('utf-8'));self.assertEqual(hashlib.sha256(raw).hexdigest(),retained['id']['sha256'])
    def test_success_and_rejected_partial_models_remain_distinct(self):
        rows=load('observations.json');byid={r['id']:r for r in rows}
        multi={r['text']:r['observation']for r in byid['multichar']['runtime']}
        for text in ['a','b','c']:self.assertTrue(multi[text]['resource']['accepted']);self.assertEqual(multi[text]['resource']['model'][0]['values']['name'],text)
        self.assertFalse(multi['ab']['resource']['accepted']);self.assertEqual(multi['ab']['resource']['model'][0]['values']['name'],'a')
        corrupted=copy.deepcopy(rows);partial=next(r for r in corrupted[1]['runtime']if r['text']=='ab');partial['observation']['resource']['accepted']=True
        with self.assertRaisesRegex(ValueError,'Rejected partial'):verifier.validate(corrupted,load('inputs/cases.json'),load('positive-resources.json'))
    def test_exact_utf16_error_tokens_and_overlapping_until_are_not_normalized(self):
        rows={c['id']:c for c in load('observations.json')};astral=next(r['observation']for r in rows['astral_literal']['runtime']if r['text']=='🌍')
        self.assertEqual([t['text']for t in astral['lexer']['tokens'][:-1]],['\ud83c','\udf0d']);self.assertEqual(astral['resource']['model'][0]['values']['name'],'\ud83c');self.assertFalse(astral['resource']['accepted'])
        for case,expected,accepted in [('abc_or_b','@ab',False),('ab_or_abc','@abc',True)]:
            run=next(r['observation']for r in rows[case]['runtime']if r['text']=='@abc');self.assertEqual(run['lexer']['tokens'][0]['text'],expected);self.assertEqual(run['resource']['accepted'],accepted)
            if not accepted:self.assertEqual(run['lexer']['tokens'][1]['type'],0);self.assertEqual(run['lexer']['tokens'][1]['channel'],99);self.assertEqual(run['lexer']['antlr_syntax_error_count'],0);self.assertTrue(run['resource']['errors'])
    def test_only_actual_owned_genmodel_output_locations_can_be_canonicalized(self):
        rows=load('observations.json');root=load('provenance.json')['recorded_provider_output_root'];verifier.comparable(rows,root)
        for forged in ['D:/unrelated/provider.genmodel','relative/provider.genmodel',root+'/provider-deadbeef0000/provider.genmodel',root+'/provider-'+rows[0]['provider']['ecore_sha256'][:12]+'/other.genmodel']:
            changed=copy.deepcopy(rows);changed[0]['provider']['genmodel']=forged
            with self.assertRaises(ValueError):verifier.comparable(changed,root)
        with self.assertRaises(ValueError):verifier.comparable(rows,'D:/unrelated/owned-output')
    def test_v58_before_preserves_unsupported_successful_grammars_and_older_profiles(self):
        before=load('before-v58.json');self.assertEqual(before['baseline_wasm_sha256'],'f7a8cc16c414c47953641394bcd65427a193d2b89ac76a7cb17fc44800221062');self.assertEqual(len(before['v58']),7);self.assertEqual(len(before['old_profiles']),116)
        self.assertEqual([r['id']for r in before['v58']if r['result']['ok']],['nested_groups']);self.assertTrue(all(not r['result']['ok']for r in before['old_profiles']))
        self.assertTrue(all(r['request']['xtext_profile']=='intersect.xtext.native.v58'for r in before['v58']))
if __name__=='__main__':unittest.main()
