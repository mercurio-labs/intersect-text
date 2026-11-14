"""Immutable independent entry-context evidence. Java is verification-only."""
import argparse,hashlib,importlib.util,json,subprocess,sys
from pathlib import Path
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent
COMPONENTS={"cases.json","observations.json","assemble.py","replay_entries.py","ScalarEntryOracle.java","verify.py","README.md"}
def checked(path,digest):
 data=path.read_bytes()
 if hashlib.sha256(data).hexdigest()!=digest:raise ValueError("SHA-256 mismatch: "+str(path))
 return data

def validate(cases,observed):
 cases=cases["cases"];metadata=observed["metadata"];generation=observed["generation"]
 assert len(cases)==10
 assert [c["id"] for c in cases]==[r["id"] for r in metadata]==[r["id"] for r in generation]
 for case,meta in zip(cases,metadata):
  assert meta["imported_structure_unchanged"] and meta["imported_metadata_unchanged"]
  assert meta["imported_before"]==meta["imported_after"]
  assert case["author_expected_metadata_acceptance"]==meta["all_sources_accepted"]
  uris=set()
  for source in case["sources"]:
   assert hashlib.sha256(source["source"].encode("utf-8")).hexdigest()==source["sha256"]
   assert source["uri"]==f'memory:/ordinary-parameters/{case["id"]}/{source["id"]}.xtext'
   uris.add(source["uri"])
  for model in case["metamodels"]:
   assert hashlib.sha256(model["source"].encode("utf-8")).hexdigest()==model["sha256"]
  def refs(value):
   if isinstance(value,dict):
    for key,item in value.items():
     if key=="uri" and isinstance(item,str) and item.startswith("memory:") and "#" in item:assert item.split("#")[0] in uris
     refs(item)
   elif isinstance(value,list):
    for item in value:refs(item)
  refs(meta)
 assert sum(r["all_sources_accepted"] for r in metadata)==9
 assert sum(r["stage"]=="compiled" for r in generation)==7
 assert {r["id"] for r in generation if r["stage"]=="generator_failed"}=={"parameterized_scalar_entry","scalar_unassigned_fragment"}
 assert {r["id"] for r in generation if r["stage"]=="metadata_rejected"}=={"scalar_via_fragment"}
 for row in generation:
  if row["stage"]!="metadata_rejected":assert row["provider"]["ecore_hashes_match_exact_imported_inputs"] and row["generated_ecore_files"]==[]
  if row["stage"]=="generator_failed":assert any("without production rules" in s for s in row["diagnostics"])
 runs=observed["runs"];assert len(runs)==38
 assert sum(r["accepted"] for r in runs)==16
 empty=lambda r:r.get("exception") is None and r.get("errors")==[] and r["model"] is None
 assert sum(empty(r) for r in runs)==3
 assert sum(not r["accepted"] and not empty(r) for r in runs)==19
 byid={r["id"]:r for r in generation}
 for case in cases:
  actual=[(r["entry"],r["text"]) for r in runs if r["case"]==case["id"]]
  assert actual==([(r["entry"],r["text"]) for r in case["runs"]] if byid[case["id"]]["stage"]=="compiled" else [])
 direct=observed["direct_entries"];named={(r["case"],r["entry"],r["text"]):r for r in runs if r["entry"]!="resource"}
 assert len(direct)==len(named)==25
 assert len({(r["case"],r["entry"],r["text"]) for r in direct})==25
 for raw in direct:
  normal=named[(raw["case"],raw["entry"],raw["text"])]
  assert raw["mode"]=="direct_generated_entry"
  assert raw["entry_methods"]==byid[raw["case"]]["entry_methods"]
  if normal.get("exception") is None:
   assert raw.get("exception") is None and raw["errors"]==normal["errors"]
   if normal["model"] is not None:assert raw["value_kind"]=="model" and raw["value"]==normal["model"]
   else:assert raw["value_kind"] in {"null","string"}
  else:
   assert raw.get("exception") and "entryRule"+raw["entry"] not in [m["method"] for m in raw["entry_methods"]]
 # A real mismatched-token error can leave ANTLR's counter at zero. Never
 # substitute that counter for the complete node-model error observation.
 assert any(r.get("errors") and r.get("antlr_syntax_error_counter")==0 for r in direct)
 assert sum(r.get("value_kind")=="string" and not r.get("errors") and not r.get("exception") for r in direct)==3
 assert len(observed["unregistered_context"])==7
 unexecuted=observed["unexecuted_requests"];assert sum(len(r["requests"]) for r in unexecuted)==18
 for row in unexecuted:
  case=next(c for c in cases if c["id"]==row["case"])
  assert row["requests"]==case["runs"] and row["reason"]==byid[row["case"]]["stage"]
 return {"graphs":10,"metadata_accepted":9,"metadata_rejected":1,"compiled_parsers":7,"rooted":16,"rejected":19,"error_free_rootless":3,"direct_entries":25,"unexecuted":18,"native_support":False}

def frozen(repo):
 p=json.loads((HERE/"provenance.json").read_text(encoding="utf-8"))
 assert set(p["components"])==COMPONENTS
 assert {f.name for f in HERE.iterdir() if f.is_file()}==COMPONENTS|{"provenance.json"}
 for name,digest in p["components"].items():checked(HERE/name,digest)
 for name,digest in p["shared_components"].items():checked(repo/name,digest)
 cases=json.loads((HERE/"cases.json").read_text(encoding="utf-8"));observed=json.loads((HERE/"observations.json").read_text(encoding="utf-8"))
 return p,cases,observed,validate(cases,observed)

def replay(repo,output,jdk,cache,expected):
 output=output.resolve();protected=repo/"fixtures"
 assert output!=protected and protected not in output.parents and output!=HERE and HERE not in output.parents
 output.mkdir(exist_ok=False,parents=True)
 def run(command,log,allowed=(0,)):
  result=subprocess.run(command,capture_output=True,text=True,encoding="utf-8")
  log.write_text(result.stdout+result.stderr,encoding="utf-8",newline="\n")
  assert result.returncode in allowed,(command[0],result.returncode,str(log))
 old=repo/"fixtures/xtext/imported-parameters"
 run([sys.executable,"-B",str(old/"replay_metadata.py"),str(HERE/"cases.json"),str(output/"metadata"),str(jdk),str(cache)],output/"metadata.log")
 run([sys.executable,"-B",str(old/"replay_runtime.py"),str(HERE/"cases.json"),str(output/"metadata/observations.json"),str(output/"runtime"),str(jdk),str(cache)],output/"runtime.log",(0,1))
 run([sys.executable,"-B",str(HERE/"replay_entries.py"),str(repo),str(HERE/"cases.json"),str(output/"runtime"),str(output/"entries"),str(jdk),str(cache)],output/"entries.log")
 spec=importlib.util.spec_from_file_location("entry_context_assembly",HERE/"assemble.py");module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
 actual=module.assemble(HERE/"cases.json",output/"metadata/observations.json",output/"runtime",output/"entries/observations.json")
 (output/"actual.json").write_text(json.dumps(actual,indent=2,ensure_ascii=False)+"\n",encoding="utf-8",newline="\n")
 assert actual==expected,"Fresh independent observations differ; expectations were not changed"
 (output/"receipt.json").write_text(json.dumps({"state":"read_only_replay_passed","observations_sha256":hashlib.sha256((HERE/"observations.json").read_bytes()).hexdigest(),"native_support":False},indent=2)+"\n",encoding="utf-8",newline="\n")

def main():
 parser=argparse.ArgumentParser();parser.add_argument("--root",type=Path,default=Path(__file__).resolve().parents[3]);parser.add_argument("--replay",type=Path);parser.add_argument("--java-home",type=Path);parser.add_argument("--maven-cache",type=Path)
 args=parser.parse_args();repo=args.root.resolve();_,_,expected,report=frozen(repo)
 if args.replay:
  if not args.java_home or not args.maven_cache:parser.error("Replay requires --java-home and --maven-cache")
  replay(repo,args.replay,args.java_home.resolve(),args.maven_cache.resolve(),expected);report["read_only_replay"]=True
 print(json.dumps(report))
if __name__=="__main__":main()
