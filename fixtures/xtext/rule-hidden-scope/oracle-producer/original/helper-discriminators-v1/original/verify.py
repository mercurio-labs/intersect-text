"""Portable full generated Xtext evidence integrity and optional independent replay."""
import argparse,copy,hashlib,json,subprocess,sys
from pathlib import Path,PureWindowsPath,PurePosixPath
FIXTURE=Path(__file__).resolve().parent

def digest(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def load(path):return json.loads(path.read_text(encoding='utf-8'))
def comparable(rows,output_root):
    result=copy.deepcopy(rows)
    for row in result:
        raw=row['provider']['genmodel'];kind=PureWindowsPath if PureWindowsPath(raw).drive else PurePosixPath
        path=kind(raw);root=kind(str(output_root));digest_=row['provider']['ecore_sha256']
        if not path.is_absolute()or not root.is_absolute()or '..'in path.parts or '..'in root.parts:raise ValueError('Provider GenModel is not an absolute owned output path')
        if len(digest_)!=64 or any(c not in '0123456789abcdef'for c in digest_):raise ValueError('Provider Ecore SHA256 differs')
        expected=root/('provider-'+digest_[:12])/'provider.genmodel'
        if path!=expected:raise ValueError('Provider GenModel lies outside the exact owned provider output')
        row['provider']['genmodel']='<provider-output>/provider.genmodel'
    return result

def validate(rows,cases,positive):
    if [r['id']for r in rows]!=[c['id']for c in cases]:raise ValueError('Full original graph order/identity differs')
    expected=[]
    for case,row in zip(cases,rows):
        if row['original_inputs']!=case['inputs']or [r['text']for r in row['runtime']]!=case['inputs']:raise ValueError('Runtime request graph incomplete')
        if row['generation']!={'exit_code':0,'source_unchanged':True}or row['compilation']!={'exit_code':0}:raise ValueError('Full generation/compilation stage differs')
        if row['provider']['generation_exit']!=0 or row['provider']['compile_exit']!=0:raise ValueError('Actual original Ecore provider missing')
        for run in row['runtime']:
            if run['exit_code']!=0 or run['timeout_seconds']is not None:raise ValueError('Runtime process did not complete')
            full=run['observation'];resource=full['resource'];parser=full['parser']
            if full['text']!=run['text']or not full['lexer']['through_eof']:raise ValueError('Full runtime request/token closure differs')
            accepted=resource['model']is not None and not resource['errors']and resource['failure']is None
            if accepted!=resource['accepted']or parser['accepted']!=accepted:raise ValueError('Rejected partial/error resource mislabeled as successful')
            if accepted:expected.append({'id':case['id'],'text':run['text'],'resource':resource})
    if expected!=positive:raise ValueError('Actual successful resource/model projection differs')
    counts={'graphs':len(rows),'runtime_inputs':sum(len(r['runtime'])for r in rows),'accepted':len(expected),'rejected':sum(not run['observation']['resource']['accepted']for r in rows for run in r['runtime']),'rejected_partial':sum(not run['observation']['resource']['accepted']and run['observation']['resource']['model']is not None for r in rows for run in r['runtime'])}
    if counts!={'graphs':7,'runtime_inputs':80,'accepted':27,'rejected':53,'rejected_partial':29}:raise ValueError('Original complete evidence denominator differs')
    return counts

def frozen():
    provenance=load(FIXTURE/'provenance.json')
    for name,h in provenance['files'].items():
        if digest(FIXTURE/name)!=h:raise ValueError('Frozen bytes differ: '+name)
    for name,h in load(FIXTURE/'inputs/pins.json').items():
        if digest(FIXTURE/'inputs'/name)!=h:raise ValueError('Original source graph differs: '+name)
    return validate(load(FIXTURE/'observations.json'),load(FIXTURE/'inputs/cases.json'),load(FIXTURE/'positive-resources.json'))

def main():
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--replay',type=Path);parser.add_argument('--jdk',type=Path);parser.add_argument('--maven-cache',type=Path);args=parser.parse_args();counts=frozen()
    if args.replay:
        if args.jdk is None or args.maven_cache is None:parser.error('Replay requires explicit --jdk and --maven-cache')
        subprocess.run([sys.executable,'-B',str(FIXTURE/'produce.py'),'--output',str(args.replay.resolve()),'--jdk',str(args.jdk.resolve()),'--maven-cache',str(args.maven_cache.resolve())],check=True)
        if comparable(load(args.replay/'observations.json'),args.replay.resolve())!=comparable(load(FIXTURE/'observations.json'),load(FIXTURE/'provenance.json')['recorded_provider_output_root']):raise ValueError('Complete full generated-language replay differs')
        frozen()
    print(json.dumps({'state':'verified','counts':counts,'native_award':False}))
if __name__=='__main__':main()
