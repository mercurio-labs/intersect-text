"""Portable byte/source-lineage integrity; not an execution qualification award."""
from pathlib import Path
import hashlib, json
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
def validate_graphs(cases):
    for case in cases:
        request=case['request'];sources=request['bundle']['sources']
        assert sources
        ids=set()
        for source in sources:
            identity=source['id'];key=(identity['uri'],identity['sha256'])
            assert key not in ids;ids.add(key)
            assert hashlib.sha256(source['text'].encode('utf-8')).hexdigest()==identity['sha256']
        for source in sources:
            assert all((d['uri'],d['sha256'])in ids for d in source['dependencies'])
        assert (request['entry']['uri'],request['entry']['sha256'])in ids
def verify():
    manifest=json.loads((HERE/'provenance.json').read_text(encoding='utf-8'))
    for row in manifest['files']:
        p=(ROOT/row['path']).resolve();p.relative_to(ROOT)
        b=p.read_bytes();assert len(b)==row['bytes'] and hashlib.sha256(b).hexdigest()==row['sha256'],row['path']
    cases=json.loads((HERE/'requests.json').read_text(encoding='utf-8'));validate_graphs(cases);assert len(cases)==6
    before=json.loads((HERE/'before-session.json').read_text(encoding='utf-8'));assert len(before['rows'])==54
    assert {r['id']for r in before['rows']}=={c['id']for c in cases}
    selected=json.loads((HERE/'lifecycle-cases.json').read_text(encoding='utf-8'));validate_graphs(selected);assert len(selected)==2
    for c in selected:
        original=next(x for x in cases if x['id']==c['id']);assert c['request']==original['request']
        assert c['literal_expected_echo']==original['literal_expected_echo']
    return dict(files=len(manifest['files']),source_graphs=6,before_rows=54,lifecycle_graphs=2)
if __name__=='__main__':print(json.dumps(verify()))
