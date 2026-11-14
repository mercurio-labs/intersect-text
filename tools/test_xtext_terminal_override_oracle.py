"""Portable evidence integrity for the independent binding oracle."""
import hashlib
import json
from pathlib import Path
import unittest
class TerminalOverrideOracle(unittest.TestCase):
    def test_provenance_and_case_boundaries(self):
        f=Path(__file__).resolve().parents[1]/'fixtures/xtext/terminal-overrides'
        record=json.loads((f/'provenance.json').read_text(encoding='utf-8'))
        for name,digest in record['files'].items():
            self.assertEqual(hashlib.sha256((f/name).read_bytes()).hexdigest(),digest,name)
        original=json.loads((f.parent/'syntax-ast/provenance.json').read_text(encoding='utf-8'))
        self.assertEqual(record['jars'],original['jars'])
        rows=json.loads((f/'observations.json').read_text(encoding='utf-8'))
        self.assertEqual([r['case'] for r in rows],['terminal','fragment','promoted-fragment','explicit-parent-call','three-level'])
        self.assertEqual(rows[3]['terminal_call'],'audit.Base::LETTER')
        self.assertEqual(rows[3]['parser_call'],'audit.Child::LETTER')
        self.assertEqual(rows[4]['hidden'],'audit.Top::LETTER')

    def test_explicit_scope_provenance(self):
        f=Path(__file__).resolve().parents[1]/'fixtures/xtext/terminal-overrides'
        record=json.loads((f/'scope-provenance.json').read_text(encoding='utf-8'))
        for name,digest in record['files'].items():
            self.assertEqual(hashlib.sha256((f/name).read_bytes()).hexdigest(),digest,name)
        original=json.loads((f/'provenance.json').read_text(encoding='utf-8'))
        self.assertEqual(record['jars'],original['jars'])
        rows=json.loads((f/'scope-observations.json').read_text(encoding='utf-8'))
        self.assertEqual(len(rows),9)
        self.assertEqual([r['selected'] for r in rows[:4]],['audit.Base::LETTER']*4)
        self.assertEqual([r['selected'] for r in rows[4:6]],['audit.Child::LETTER']*2)
        self.assertTrue(all(r['selected'] is None for r in rows[6:]))

    def test_parent_cardinality_provenance(self):
        f=Path(__file__).resolve().parents[1]/'fixtures/xtext/terminal-overrides'
        record=json.loads((f/'inheritance-provenance.json').read_text(encoding='utf-8'))
        for name,digest in record['files'].items():
            self.assertEqual(hashlib.sha256((f/name).read_bytes()).hexdigest(),digest,name)
        self.assertEqual(record['jars'],json.loads((f/'provenance.json').read_text(encoding='utf-8'))['jars'])
        rows=json.loads((f/'inheritance-observations.json').read_text(encoding='utf-8'))
        self.assertEqual([r['parents'] for r in rows],[0,1,2,3])
        self.assertTrue(all(r['syntax_accepted'] for r in rows))
        self.assertEqual([r['error_indices'] for r in rows],[[],[],[1],[1,2]])

    def test_final_override_provenance(self):
        f=Path(__file__).resolve().parents[1]/'fixtures/xtext/terminal-overrides'
        record=json.loads((f/'final-provenance.json').read_text(encoding='utf-8'))
        for name,digest in record['files'].items():
            self.assertEqual(hashlib.sha256((f/name).read_bytes()).hexdigest(),digest,name)
        self.assertEqual(record['jars'],json.loads((f/'provenance.json').read_text(encoding='utf-8'))['jars'])
        rows=json.loads((f/'final-observations.json').read_text(encoding='utf-8'))
        self.assertEqual([(r['kind'],r['case']) for r in rows],[(k,c) for k in ['parser','terminal','enum'] for c in ['ordinary','final','final-unannotated','final-ancestor']])
        self.assertEqual([r['errors'] for r in rows],[0,1,1,1]*3)
        self.assertEqual([r['warnings'] for r in rows],[0]*12)

    def test_metamodel_selection_provenance(self):
        f=Path(__file__).resolve().parents[1]/'fixtures/xtext/terminal-overrides'
        record=json.loads((f/'metamodel-provenance.json').read_text(encoding='utf-8'))
        for name,digest in record['files'].items():
            self.assertEqual(hashlib.sha256((f/name).read_bytes()).hexdigest(),digest,name)
        self.assertEqual(record['jars'],json.loads((f/'provenance.json').read_text(encoding='utf-8'))['jars'])
        rows=json.loads((f/'metamodel-observations.json').read_text(encoding='utf-8'))
        self.assertEqual([r['case'] for r in rows],['shared-alias-unique','shared-alias-conflict','distinct-alias','missing-alias','missing-classifier','unaliased','duplicate-uri'])
        self.assertEqual([r['selected'] for r in rows],['urn:a',None,'urn:a',None,None,'urn:a','urn:a'])
        self.assertEqual([r['uri_errors'] for r in rows],[0,0,0,0,0,0,2])

    def test_category_override_provenance(self):
        f=Path(__file__).resolve().parents[1]/'fixtures/xtext/terminal-overrides'
        record=json.loads((f/'category-provenance.json').read_text(encoding='utf-8'))
        for name,digest in record['files'].items():
            self.assertEqual(hashlib.sha256((f/name).read_bytes()).hexdigest(),digest,name)
        self.assertEqual(record['jars'],json.loads((f/'provenance.json').read_text(encoding='utf-8'))['jars'])
        rows=json.loads((f/'category-observations.json').read_text(encoding='utf-8'))
        expected=[(p,c,'same-type') for p in ['parser','enum','terminal'] for c in ['parser','enum','terminal']]
        expected += [('parser','fragment','same-type'),('enum','fragment','same-type'),('parser','terminal','different-type'),('parser','terminal','missing-type')]
        self.assertEqual([(r['parent'],r['child'],r['case']) for r in rows],expected)
        self.assertEqual([r['accepted'] for r in rows],[True]*6+[False,False,True]+[False]*4)
