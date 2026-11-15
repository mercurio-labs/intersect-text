"""Build explicit local worktree consumers and compare unchanged verified graphs serially."""
import argparse
import hashlib
import importlib.util
import json
import os
import platform
import re
from datetime import datetime, timezone
from pathlib import Path
import shutil
import subprocess

HERE=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()

def runtime_digest(root):
    digest=hashlib.sha256()
    for path in sorted((root/'crates').rglob('*.rs')):
        digest.update(path.relative_to(root).as_posix().encode()+b'\0'+path.read_bytes()+b'\0')
    return digest.hexdigest()
def revision(root):
    return {'revision':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),
            'status':subprocess.check_output(['git','status','--short'],cwd=root,text=True),
            'diff_sha256':hashlib.sha256(subprocess.check_output(['git','diff','HEAD'],cwd=root)).hexdigest()}
def main():
    p=argparse.ArgumentParser(description=__doc__)
    for name in ['baseline-root','model-root','text-root','native-project','java-scope-classes','output']:p.add_argument('--'+name,type=Path,required=True)
    p.add_argument('--jdk',type=Path,required=True)
    p.add_argument('--native-baseline',type=Path,help='Optional saved native executable, measured with the same host protocol')
    p.add_argument('--native-baseline-receipt',type=Path,help='Provenance JSON for the saved executable')
    p.add_argument('--java-overlay-receipt',type=Path,help='Fingerprint receipt for an optional corrected Java host/scope overlay')
    p.add_argument('--validation-contract',type=Path,help='Passed stage-aware controls bound to these runtime sources and Java classes')
    p.add_argument('--mode',choices=['text','xmi'],default='text')
    p.add_argument('--sizes',type=int,nargs='+',default=[1000,10000,100000,1000000]);p.add_argument('--degrees',type=int,nargs='+',default=[1,8]);p.add_argument('--repeats',type=int,default=5)
    p.add_argument('--check-dto',action='store_true');p.add_argument('--timeout',type=int,default=180)
    a=p.parse_args();base=a.baseline_root.resolve();model=a.model_root.resolve();text=a.text_root.resolve();project=a.native_project.resolve();out=a.output.resolve();classes=a.java_scope_classes.resolve()
    if min(a.sizes)<1 or a.repeats<1 or a.timeout<=0:p.error('sizes, repeats and timeout must be positive')
    if bool(a.native_baseline)!=bool(a.native_baseline_receipt):p.error('provide both native baseline and its receipt')
    if a.mode=='xmi' and a.check_dto:p.error('--check-dto cross-checks the Text live-store verifier only')
    baseline_receipt=None
    validation_contract=None
    java_overlay=None
    if a.java_overlay_receipt:
        java_overlay=json.loads(a.java_overlay_receipt.read_text())
        assert sha(base/'Benchmark.java')==java_overlay['original_host_sha256']
        for name,pin in java_overlay['classes'].items():assert sha(classes/name)==pin,name
    if a.validation_contract:
        validation_contract=json.loads(a.validation_contract.read_text())
        assert validation_contract.get('mode','text')==a.mode,'Contract mode differs from measurement'
        assert validation_contract['common_rejections_passed'] and validation_contract['runtime_build_patch_verified']
        assert java_overlay and java_overlay['classes']==validation_contract['java_overlay']['classes']
        assert sha(HERE/'native.rs')==validation_contract['native_provenance']['native_host_sha256']
        assert all(runtime_digest(root)==validation_contract['runtime_source_digests'][name] for name,root in [('model',model),('text',text)]),'Runtime changed since contract controls'
    if a.native_baseline:
        baseline_receipt=json.loads(a.native_baseline_receipt.read_text())
        if sha(a.native_baseline)!=baseline_receipt['native_binary_sha256']:p.error('native baseline fingerprint differs from its receipt')
    out.mkdir(parents=True,exist_ok=False);(project/'src').mkdir(parents=True,exist_ok=True)
    sources={name:revision(root) for name,root in [('model',model),('text',text)]}
    for name,root in [('model',model),('text',text)]:
        untracked=subprocess.check_output(['git','ls-files','--others','--exclude-standard','--','crates'],cwd=root,text=True)
        if untracked.strip():raise RuntimeError(f'Stage new {name} crate sources before benchmarking: {untracked}')
        patch=out/(name+'.diff');patch.write_bytes(subprocess.check_output(['git','diff','HEAD'],cwd=root))
        assert sha(patch)==sources[name]['diff_sha256']
    shutil.copyfile(HERE/'native.rs',project/'src/main.rs')
    path=lambda root,crate:json.dumps((root/'crates'/crate).as_posix())
    manifest='''[package]
name="sysml-scale-benchmark"
version="0.1.0"
edition="2024"
[dependencies]
intersect-model={path=MODEL,features=["xmi"]}
intersect-text={path=TEXT}
serde_json="1.0.149"
sha2="0.10.9"
[patch."https://github.com/mercurio-labs/intersect-model.git"]
intersect-model={path=MODEL}
intersect-build-support={path=SUPPORT}
[workspace]
'''.replace('MODEL',path(model,'intersect-model')).replace('TEXT',path(text,'intersect-text')).replace('SUPPORT',path(model,'intersect-build-support'))
    (project/'Cargo.toml').write_text(manifest,encoding='utf-8')
    if not (project/'Cargo.lock').exists():shutil.copyfile(base/'native/Cargo.lock',project/'Cargo.lock')
    build=['cargo','+1.99.1','build','--release','--locked','--offline','--manifest-path',str(project/'Cargo.toml')]
    built=subprocess.run(build,cwd=text,capture_output=True,text=True)
    (out/'build.stdout.log').write_text(built.stdout,encoding='utf-8');(out/'build.stderr.log').write_text(built.stderr,encoding='utf-8');built.check_returncode()
    assert all(revision(root)==sources[name] for name,root in [('model',model),('text',text)]),'Source changed during build'
    binary=project/'target/release/sysml-scale-benchmark.exe'
    spec=importlib.util.spec_from_file_location('baseline_runner',base/'run.py');runner=importlib.util.module_from_spec(spec);spec.loader.exec_module(runner)
    original=runner.command
    def command(profile,folder,manifest):
        if profile.startswith('intersect-'):
            executable=a.native_baseline.resolve() if profile.endswith('-before') else binary
            cmd=[str(executable),a.mode,str(folder),str(manifest['objects']),manifest['checksum']]
            if a.check_dto:cmd.append('--check-dto')
        else:
            cmd=original(profile,folder,manifest)
            cmd[0]=str(a.jdk.resolve()/'bin/java.exe')
            cmd[cmd.index('-cp')+1]=str(classes)+os.pathsep+cmd[cmd.index('-cp')+1]
        if a.mode=='xmi':cmd.append('--fixture-identities')
        return cmd
    runner.command=command
    original_report=runner.report
    def report(out,data):
        original_report(out,data)
        p=out/'REPORT.md';s=p.read_text(encoding='utf-8')
        s=s.replace('three first/second-load pairs per case by default',f"{a.repeats} first/second-load pairs per case")
        s=s.replace('before correctness export/hashing','before correctness hashing')
        if a.mode=='text':s+='\nCandidate text verification reads the live store; no full compatibility snapshot is exported'+(' except in the explicit DTO cross-check control.\n' if a.check_dto else '.\n')
        else:s+='\nXMI uses the public owned snapshot decoder. Snapshot materialization is included in loading; hashing is outside the timed phase. No compact-XMI parity is claimed.\n'
        if a.repeats<5:s+='This is a smoke comparison, not five-process parity qualification.\n'
        p.write_text(s,encoding='utf-8')
    runner.report=report
    native_profile='intersect-text' if a.mode=='text' else 'intersect-xmi'
    java_profile='xtext' if a.mode=='text' else 'emf-xmi'
    data={'provenance':{'utc':datetime.now(timezone.utc).isoformat(),'platform':platform.platform(),'machine':platform.node(),'cpu_count':os.cpu_count(),'java':subprocess.run([str(a.jdk.resolve()/'bin/java.exe'),'-version'],capture_output=True,text=True).stderr,'model':revision(model),'text':revision(text),'native_binary_sha256':sha(binary),'native_host_sha256':sha(HERE/'native.rs'),'runner_sha256':sha(Path(__file__)),'scope_class_sha256':sha(classes/'bench/FlatScopeProvider.class'),'baseline_runner_sha256':sha(base/'run.py'),'java_runtime_classpath_sha256':sha(base/'build/runtime-classpath.txt'),'java_dependency_pins':json.loads((base/'build/dependency-pins.json').read_text()),'build_command':build,'consumer_manifest':manifest,'consumer_lock_sha256':sha(project/'Cargo.lock'),'verification':('borrowed store'+(' plus DTO cross-check' if a.check_dto else '')) if a.mode=='text' else 'owned XMI snapshot'},
          'mode':a.mode,'sizes':a.sizes,'degrees':a.degrees,'profiles':([native_profile+'-before'] if a.native_baseline else [])+[native_profile,java_profile],'repeats':a.repeats,'phase_timeout_seconds':a.timeout,'input_manifests':{},'runs':[]}
    if baseline_receipt:data['provenance']['native_baseline']=baseline_receipt
    if java_overlay:data['provenance']['java_overlay']=java_overlay
    if validation_contract:data['provenance']['validation_contract']={'sha256':sha(a.validation_contract),'runs':len(validation_contract['runs']),'runtime_source_digests':validation_contract['runtime_source_digests']}
    data['provenance']['source_patches']={name+'.diff':sha(out/(name+'.diff')) for name in sources}
    for n in a.sizes:
      for degree in a.degrees:
        folder=base/'inputs'/f'n{n}-d{degree}';m=json.loads((folder/'manifest.json').read_text());data['input_manifests'][folder.name]=m
        for name,pin in m['files'].items():assert sha(folder/name)==pin['sha256']
        for repeat in range(a.repeats):
          for profile in data['profiles'][::1 if repeat%2==0 else -1]:
            dest=out/f'{folder.name}-{profile}-r{repeat}';dest.mkdir();print('START',dest.name,flush=True)
            row=runner.once(profile,folder,m,dest,a.timeout)
            if a.mode=='xmi' and row['status']=='ok':
                assert re.findall(r'FIXTURE_IDENTITIES_VERIFIED (\d+)',row['stderr_tail'])==[str(n),str(n)],'Missing fixture identity receipts'
            row.update(repeat=repeat,directory=dest.name);data['runs'].append(row);runner.report(out,data)
            elapsed=next((e['pipeline_ms'] for e in row['events'] if e['phase']=='cold_loaded'),None)
            print('END',dest.name,row['status'],elapsed,flush=True)
    m=json.loads((base/'inputs/n1000-d1/manifest.json').read_text());controls=[]
    for mutation in ['attribute','reference']:
      for profile in data['profiles']:
        dest=out/f'negative-{mutation}-{profile}';dest.mkdir();r=runner.once(profile,base/'inputs'/('negative-'+mutation),m,dest,45)
        assert r['status']=='failed' and any(e['phase']=='inputs' for e in r['events']),r
        if mutation=='attribute':assert 'Graph checksum' in r['stderr_tail']
        controls.append({'mutation':mutation,'profile':profile,'correctly_rejected':True})
    (out/'negative-controls.json').write_text(json.dumps(controls,indent=2)+'\n',encoding='utf-8')
    data['provenance']['sources_unchanged']=all(revision(root)==sources[name] for name,root in [('model',model),('text',text)])
    runner.report(out,data)
    assert data['provenance']['sources_unchanged'],'Source changed during measurements'
    print('COMPLETE',out/'REPORT.md')
if __name__=='__main__':main()
