"""Frozen source/oracle lineage controls; no Java unless explicitly requested elsewhere."""
import hashlib
import json
import os
import subprocess
import sys
import unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
FIXTURE=ROOT/'fixtures/xtext/terminal-cardinality'
NATIVE=ROOT/'crates/intersect-web/tests/fixtures/terminal-cardinality-v63'
def disk(p):
    p=Path(p).resolve()
    return p if os.name!='nt' or str(p).startswith('\\\\?\\') else Path('\\\\?\\'+str(p))
def read(p):return json.loads(disk(p).read_text(encoding='utf-8'))
class TerminalCardinalityEvidence(unittest.TestCase):
    def test_complete_portable_bytes_original_stage_pairs_and_history(self):
        result=subprocess.run([sys.executable,'-B',str(FIXTURE/'verify.py')],capture_output=True,text=True,encoding='utf-8')
        self.assertEqual(result.returncode,0,result.stdout+result.stderr)
    def test_native_views_retain_every_original_xtext_source_identity_and_byte(self):
        originals={}
        for label in ['original8','typed8','fragment2','return12']:
            folder=FIXTURE/'oracle-corpus/inputs'/label/'inputs'
            for case in read(folder/'cases.json'):
                self.assertNotIn(case['id'],originals)
                originals[case['id']]=(case,folder/case['id'])
        native=read(NATIVE/'cases.json')['cases']
        self.assertEqual(len(originals),30)
        self.assertEqual({c['id'] for c in native},set(originals))
        # Dependency-context addenda stay separately named; this asserts source
        # identity equality without qualifying their original failed context.
        for c in [*native,*read(NATIVE/'context-addendum.json')['cases']]:
            case,folder=originals[c['id']]
            actual=[s for s in c['compilation']['bundle']['sources'] if s['kind']=='xtext']
            self.assertEqual({(s['id']['uri'],s['id']['sha256']) for s in actual},
                {(s['source_id']['uri'],s['source_id']['sha256']) for s in case['sources']})
            for source in case['sources']:
                raw=disk(folder/source['file']).read_bytes()
                self.assertEqual(hashlib.sha256(raw).hexdigest(),source['source_id']['sha256'])
                retained=next(s for s in actual if s['id']==source['source_id'])
                self.assertEqual(retained['text'].encode('utf-8'),raw)
            entry=next(s for s in case['sources'] if s['grammar']==case['entry'])
            self.assertEqual(c['compilation']['xtext'],entry['source_id'])
if __name__=='__main__':unittest.main()
