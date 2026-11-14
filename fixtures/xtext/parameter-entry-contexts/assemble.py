"""Canonicalize independently captured entry contexts, retaining stage differences."""
from pathlib import Path
import copy,hashlib,json,re

def assemble(plan_path,metadata_path,capture,entry_path):
 plan_path,metadata_path,capture,entry_path=map(Path,(plan_path,metadata_path,capture,entry_path))
 plan=json.loads(plan_path.read_text(encoding="utf-8"));metadata=json.loads(metadata_path.read_text(encoding="utf-8"))
 ledger=json.loads((capture/"capture-ledger.json").read_text(encoding="utf-8"))
 direct=json.loads(entry_path.read_text(encoding="utf-8"))
 rows={r["id"]:r for r in ledger["results"]};generations=[];runs=[];unregistered=[];unexecuted=[]
 assert [r["id"] for r in metadata]==[c["id"] for c in plan["cases"]]
 for case,meta in zip(plan["cases"],metadata):
  row=rows[case["id"]];stage=row["state"]
  assert meta["imported_structure_unchanged"] and meta["imported_metadata_unchanged"]
  if not meta["all_sources_accepted"]:
   assert stage=="metadata_rejected"
   generations.append({"id":case["id"],"stage":"metadata_rejected"})
   unexecuted.append({"case":case["id"],"reason":"metadata_rejected","requests":case["runs"]});continue
  project=capture/"languages"/case["id"]
  for source in case["sources"]:
   f=project/"src"/Path(source["grammar"].replace(".","/")+".xtext")
   assert hashlib.sha256(f.read_bytes()).hexdigest()==source["sha256"]
  assert not list((project/"model/generated").glob("*.ecore"))
  assert row["provider"]["ecore_hashes_match_exact_imported_inputs"]
  if stage=="capture_failed_unqualified":
   assert row["generation"]["exit_code"]!=0
   assert case["id"] in {"parameterized_scalar_entry","scalar_unassigned_fragment"}
   log=(project/"generation.log").read_text(encoding="utf-8")
   diagnostics=[]
   for line in log.splitlines():
    line=line.strip()
    if "You may not generate an ANTLR parser for a grammar without production rules." in line or ("ClassCastException:" in line and "EDataTypeImpl" in line):
     line=re.sub(r"^Caused by: ","",line)
     if line not in diagnostics:diagnostics.append(line)
   assert any("without production rules" in d for d in diagnostics)
   assert not row["runs"]
   generations.append({"id":case["id"],"stage":"generator_failed","provider":row["provider"],"exit_code":row["generation"]["exit_code"],"diagnostics":sorted(diagnostics),"generated_ecore_files":[]})
   unexecuted.append({"case":case["id"],"reason":"generator_failed","requests":case["runs"]});continue
  assert stage=="captured"
  own=[r for r in direct if r["case"]==case["id"]]
  assert own and all(r["entry_methods"]==own[0]["entry_methods"] for r in own)
  generations.append({"id":case["id"],"stage":"compiled","provider":row["provider"],"entry_methods":own[0]["entry_methods"],"generated_ecore_files":[]})
  assert [(r["entry"],r["text"]) for r in row["runs"]]==[(r["entry"],r["text"]) for r in case["runs"]]
  for r in row["runs"]:runs.append({"case":case["id"],**r})
  unregistered.append({"case":case["id"],**row["unregistered_context_probe"]})
 return {"version":1,"metadata":metadata,"generation":generations,"runs":runs,"direct_entries":direct,"unregistered_context":unregistered,"unexecuted_requests":unexecuted}
