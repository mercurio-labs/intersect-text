"""Portable byte/source/observation integrity for durable document evidence."""
from pathlib import Path
import importlib.util
import os
import subprocess
import sys
import tempfile
import unittest

class DurableDocumentEvidence(unittest.TestCase):
    def test_complete_portable_evidence_and_storage_negatives(self):
        root=Path(__file__).resolve().parents[1]
        verifier=root/'fixtures/xtext/durable-documents/verify.py'
        spec=importlib.util.spec_from_file_location('durable_frozen_inventory',verifier)
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        corpus=module.Corpus()
        # Keep the original frozen consumer names and bytes. This temporary
        # evidence view maps the installed renamed package to those historical
        # paths; it is not a compatibility crate or a production workspace.
        with tempfile.TemporaryDirectory(prefix='intersect-document-evidence-') as directory:
            historical=Path(directory)
            for name in corpus.origins:
                if not name.startswith('consumer/'):
                    continue
                original=name[len('consumer/'):]
                installed=original.replace('crates/intersect-xtext/','crates/intersect-text/',1)
                source=root/installed
                if installed.startswith('toolkit/'):
                    examples=os.environ.get('INTERSECT_EXAMPLES_ROOT')
                    if not examples:
                        self.fail('Set INTERSECT_EXAMPLES_ROOT to an explicit intersect-lang-examples checkout for the relocated browser fixture comparison')
                    source=Path(examples)/installed
                target=historical/original;target.parent.mkdir(parents=True,exist_ok=True)
                target.write_bytes(source.read_bytes())
            result=subprocess.run([sys.executable,str(verifier),'--negatives','--repository',str(historical)],cwd=root,
                                  stdout=subprocess.PIPE,stderr=subprocess.PIPE)
        self.assertEqual(result.returncode,0,
                         result.stdout.decode('utf-8',errors='strict')+
                         result.stderr.decode('utf-8',errors='strict'))

if __name__=='__main__':
    unittest.main()
