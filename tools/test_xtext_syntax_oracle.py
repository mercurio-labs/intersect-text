"""Portable integrity for the independent syntax parser observations."""
import hashlib
import json
from pathlib import Path
import unittest

class SyntaxOracleIntegrity(unittest.TestCase):
    def test_pins_and_cases(self):
        fixture = Path(__file__).resolve().parents[1]/'fixtures/xtext/syntax-ast'
        provenance = json.loads((fixture/'provenance.json').read_text(encoding='utf-8'))
        for name, digest in provenance['files'].items():
            self.assertEqual(hashlib.sha256((fixture/name).read_bytes()).hexdigest(), digest, name)
        paths = [item['path'] for item in provenance['jars']]
        self.assertEqual(len(paths), len(set(paths)))
        self.assertIn('org/eclipse/xtext/org.eclipse.xtext/2.38.0/org.eclipse.xtext-2.38.0.jar', paths)
        cases = json.loads((fixture/'cases.json').read_text(encoding='utf-8'))
        observations = json.loads((fixture/'observations.json').read_text(encoding='utf-8'))
        self.assertEqual([c['id'] for c in cases], [r['id'] for r in observations])
        self.assertEqual(len(cases), 19)
        lone = next(r for r in observations if r['id']=='isolated-surrogate')
        self.assertFalse(lone['syntax_errors'])
        self.assertEqual(next(n['attributes']['value']['utf16'] for n in lone['nodes'] if n['class']=='Keyword'), [0xd800])

    def test_additive_structure_observation(self):
        fixture=Path(__file__).resolve().parents[1]/'fixtures/xtext/syntax-ast'
        provenance=json.loads((fixture/'structure-provenance.json').read_text())
        for name,digest in provenance['files'].items():
            self.assertEqual(hashlib.sha256((fixture/name).read_bytes()).hexdigest(),digest,name)
        row=json.loads((fixture/'structure-observation.json').read_text())
        self.assertFalse(row['syntax_errors'])
        self.assertEqual(sum(n['class']=='Annotation' for n in row['nodes']),2)
        self.assertEqual(sum(n['class']=='NamedArgument' for n in row['nodes']),2)
    def test_node_matrix_covers_pinned_concrete_forms(self):
        import xml.etree.ElementTree as ET
        fixture=Path(__file__).resolve().parents[1]/'fixtures/xtext/syntax-ast'
        provenance=json.loads((fixture/'coverage-provenance.json').read_text())
        for name,digest in provenance['files'].items():
            self.assertEqual(hashlib.sha256((fixture/name).read_bytes()).hexdigest(),digest,name)
        rows=json.loads((fixture/'coverage-observations.json').read_text())
        cases=json.loads((fixture/'coverage-cases.json').read_text())
        self.assertEqual([r['id'] for r in rows],[c['id'] for c in cases])
        self.assertEqual(len(rows),11)
        self.assertFalse(rows[0]['syntax_errors'])
        observed={n['class'] for n in rows[0]['nodes']}
        self.assertEqual(len(observed),29)
        schema=fixture.parents[1]/'definition-conformance/upstream/xtext/org/eclipse/xtext/Xtext.ecore'
        pinned={c.get('name') for c in ET.parse(schema).getroot().findall('eClassifiers') if c.get('{http://www.w3.org/2001/XMLSchema-instance}type')=='ecore:EClass'}
        families={'AbstractRule','AbstractElement','AbstractMetamodelDeclaration','AbstractNegatedToken','CompoundElement','Condition','CompositeCondition'}
        self.assertEqual(observed|families,pinned)
        self.assertFalse(observed&families)
    def test_complete_tree_observation_provenance(self):
        fixture=Path(__file__).resolve().parents[1]/'fixtures/xtext/syntax-ast'
        provenance=json.loads((fixture/'tree-provenance.json').read_text())
        for name,digest in provenance['files'].items():
            self.assertEqual(hashlib.sha256((fixture/name).read_bytes()).hexdigest(),digest,name)
        rows=json.loads((fixture/'tree-observations.json').read_text())
        self.assertEqual(len(rows),31)
        self.assertEqual(sum(not r['syntax_errors'] for r in rows),14)
        self.assertTrue(all(r['tree'] is None for r in rows if r['syntax_errors']))
