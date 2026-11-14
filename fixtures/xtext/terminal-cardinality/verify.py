"""Pure source/oracle/history integrity; Java replay only with explicit arguments."""
import argparse
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
from pathlib import Path,PureWindowsPath,PurePosixPath
sys.dont_write_bytecode=True
ROOT=Path(__file__).resolve().parent
def disk(p):
    p=Path(p).resolve()
    return p if os.name!='nt' or str(p).startswith('\\\\?\\') else Path('\\\\?\\'+str(p))
def pin(p):
    b=disk(p).read_bytes()
    return {'bytes':len(b),'sha256':hashlib.sha256(b).hexdigest()}
def read(p):return json.loads(disk(p).read_text(encoding='utf-8'))
def verify():
    provenance=read(ROOT/'provenance.json')
    actual=set()
    for folder,_,names in os.walk(disk(ROOT)):
        for name in names:
            relative=(Path(folder)/name).relative_to(disk(ROOT)).as_posix()
            if relative!='provenance.json':actual.add(relative)
    assert actual==set(provenance['files']),'Unexpected or missing payload files'
    for relative,expected in provenance['files'].items():assert pin(ROOT/relative)==expected,relative
    subprocess.run([sys.executable,'-B',str(ROOT/'oracle-corpus/oracle-producer/verify.py')],check=True)
    spec=importlib.util.spec_from_file_location('frozen_original_verifier',ROOT/'portable-replay/original-verifier.py')
    module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
    relocations=read(ROOT/'verification/fresh-full-preexecution.json')['relocations']
    pairs=[('original8','inputs/original8','oracle/original8'),('typed8','inputs/typed8','oracle/typed8'),('fragment2','inputs/fragment2','oracle/fragment2'),('return12','inputs/return12','oracle/return12'),('inherited2','oracle/inherited-workflow/producer-and-receipts','oracle/inherited-workflow')]
    graphs=0
    for label,inputs,outputs in pairs:
        selected=next(r for r in relocations if r['id']==label)
        value=selected['original_owned_output_root'];kind=PureWindowsPath if PureWindowsPath(value).drive else PurePosixPath
        record_root=kind(value)
        replay_root=record_root.with_name('tilw-a4-replay') if label=='inherited2' else record_root.with_name('replay')
        rows=[read(ROOT/'oracle-corpus'/outputs/key/'observations.json') for key in ['record','replay']]
        assert module.comparable(rows[0],str(record_root))==module.comparable(rows[1],str(replay_root)),label
        cases=read(ROOT/'oracle-corpus'/inputs/'inputs/cases.json');graphs+=len(cases)
        assert [r['id'] for r in rows[0]]==[c['id'] for c in cases]
        for row,case in zip(rows[0],cases):
            assert row['entry']==case['entry'] and row['original_inputs']==case['inputs']
            for source in case['sources']:
                source_file=ROOT/'oracle-corpus'/inputs/'inputs'/case['id']/source['file']
                assert pin(source_file)['sha256']==source['source_id']['sha256']
                assert row['source_pins'][source['file']]==source['source_id']['sha256']
    assert graphs==32
    historical=read(ROOT/'historical-workflows/manifest-v2.json')
    assert pin(ROOT/'historical-workflows/manifest.json')==historical['initial_immutable_inventory']
    for relative,expected in historical['files'].items():assert pin(ROOT/'historical-workflows'/relative)=={k:expected[k] for k in ['bytes','sha256']}
    assert len(historical['files'])==sum(h['files'] for h in historical['inventory'])
    for h in historical['inventory']:
        assert all(s['complete_runtime_observations']==0 for s in h['stages'])
        if h['attempt']==1:assert all('generated_javac_failure' in s['stage_labels'] and not s['runtime_attempts'] for s in h['stages'])
        else:assert all('runtime_process_failure' in s['stage_labels'] and s['runtime_exit_codes']==[1]*5 for s in h['stages'])
    print(json.dumps({'state':'complete_portable_payload_integrity_and_original_semantic_pairs_valid','payload_files':len(actual),'original_graphs':30,'distinct_inherited_workflow_graphs':2,'historical_raw_files':len(historical['files']),'java_executed':False,'gate_closures':[]}))
def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--replay',type=Path);parser.add_argument('--jdk',type=Path);parser.add_argument('--maven-cache',type=Path)
    args=parser.parse_args();verify()
    if args.replay:
        assert args.jdk and args.maven_cache,'Replay requires explicit pinned JDK and Maven cache'
        subprocess.run([sys.executable,'-B',str(ROOT/'portable-replay/replay.py'),'--corpus',str(ROOT/'oracle-corpus'),'--output',str(args.replay),'--jdk',str(args.jdk),'--maven-cache',str(args.maven_cache)],check=True)
    else:assert args.jdk is None and args.maven_cache is None,'JDK/cache supplied without replay'
if __name__=='__main__':main()
