"""Offline source integrity; optional pinned Xtext/EMF converter replay (no downloads)."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--upstream', type=Path)
    parser.add_argument('--jdk', type=Path)
    parser.add_argument('--artifact-dir', type=Path)
    parser.add_argument('--maven-cache', type=Path, default=Path.home()/'.m2/repository')
    parser.add_argument('--ecore-jar', type=Path)
    args = parser.parse_args()
    fixture = Path(__file__).resolve().parent
    provenance = json.loads((fixture/'provenance.json').read_text(encoding='utf-8'))
    for name, expected in provenance['files'].items():
        assert digest(fixture/name) == expected, name
    pin = json.loads((fixture/'source-provenance.json').read_text(encoding='utf-8'))
    if args.upstream:
        raw = (args.upstream/pin['source']).read_bytes()
        assert hashlib.sha256(raw).hexdigest() == pin['sha256']
        model = (fixture/'model.xtext').read_bytes()
        for span in pin['slices']:
            data = raw[span['start']:span['end']]
            assert hashlib.sha256(data).hexdigest() == span['sha256']
            assert model.count(data) == 1
    if args.jdk:
        if not args.artifact_dir:
            parser.error('--artifact-dir is required with --jdk')
        jars=[]
        lock=json.loads((fixture.parents[1]/'definition-conformance/upstream/artifacts.json').read_text(encoding='utf-8'))
        pinned={row.get('archive_sha256') for row in lock['files']}
        for row in provenance['jars']:
            jar=args.ecore_jar if 'org.eclipse.emf.ecore/' in row['maven_path'] and args.ecore_jar else args.maven_cache/row['maven_path']
            assert digest(jar)==row['sha256'],str(jar)
            if 'org.eclipse.emf.common/' not in row['maven_path']:
                assert row['sha256'] in pinned
            jars.append(str(jar))
        suffix='.exe' if os.name=='nt' else ''
        args.artifact_dir.mkdir(parents=True,exist_ok=True)
        cp=os.pathsep.join(jars)
        subprocess.run([str(args.jdk/'bin'/('javac'+suffix)),'-encoding','UTF-8','-cp',cp,'-d',str(args.artifact_dir),str(fixture/'DoubleOracle.java')],check=True,timeout=60)
        cases=json.loads((fixture/'cases.json').read_text(encoding='utf-8'))
        result=subprocess.run([str(args.jdk/'bin'/('java'+suffix)),'-cp',os.pathsep.join([str(args.artifact_dir),cp]),'DoubleOracle',*cases],check=True,capture_output=True,timeout=60)
        rows=[json.loads(line) for line in result.stdout.decode('ascii').splitlines()]
        assert rows==json.loads((fixture/'observations.json').read_text(encoding='utf-8'))
    print('Verified exact datatype fixture identities and 25 cached converter observations; grammar execution is a separate qualification.')


if __name__=='__main__':
    main()
