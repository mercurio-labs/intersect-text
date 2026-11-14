"""Independent direct generated-entry returns; no native compiler participation."""
from pathlib import Path
import hashlib,json,os,subprocess,sys
sys.dont_write_bytecode=True
repo,plan_path,capture,out=map(lambda s:Path(s).resolve(),sys.argv[1:5])
jdk,cache=map(lambda s:Path(s).resolve(),sys.argv[5:7])
selected=set(sys.argv[7:])
plan=json.loads(plan_path.read_text(encoding="utf-8"))
ledger=json.loads((capture/"capture-ledger.json").read_text(encoding="utf-8"))
byid={r["id"]:r for r in ledger["results"]}
out.mkdir(exist_ok=False)
source=Path(__file__).with_name("ScalarEntryOracle.java")
pin=json.loads((repo/"fixtures/xtext/inference-fragment-runtime/provenance.json").read_text(encoding="utf-8"))

jars=[]
for row in pin["jars"]:
 jar=cache/row["path"];assert hashlib.sha256(jar.read_bytes()).hexdigest()==row["sha256"]
 jars.append(str(jar))
cp=os.pathsep.join([str(capture/"adapter/classes"),*jars])
classes=out/"classes";classes.mkdir()
args=["-encoding","UTF-8","--release","17","-cp",cp,"-d",str(classes),str(source)]
argfile=out/"javac.args";argfile.write_text("\n".join('"'+a.replace('\\','/')+'"' for a in args)+"\n",encoding="utf-8",newline="\n")
r=subprocess.run([str(jdk/"bin/javac.exe"),"@"+str(argfile)],capture_output=True,text=True,encoding="utf-8",timeout=180)
(out/"compilation.log").write_text(r.stdout+r.stderr,encoding="utf-8",newline="\n");r.check_returncode()
results=[];skipped=[]
for case in plan["cases"]:
 if selected and case["id"] not in selected:continue
 row=byid.get(case["id"])
 if not row or row["state"]!="captured":
  skipped.append({"case":case["id"],"state":None if row is None else row["state"]});continue
 project=capture/"languages"/case["id"];provider=capture/"providers"/case["id"]/case["oracle_model_provider"]["scaffold_case"]["id"]
 for s in case["sources"]:
  path=project/"src"/Path(s["grammar"].replace(".","/")+".xtext")
  assert hashlib.sha256(path.read_bytes()).hexdigest()==s["sha256"]
 runtime_cp=os.pathsep.join([str(classes),str(capture/"adapter/classes"),str(project/"classes"),str(project/"src-gen"),str(project/"src"),str(provider/"classes"),str(provider/"src-gen"),str(provider/"src"),*jars])
 grammar=next(s["grammar"] for s in case["sources"] if s["id"]==case["entry"])
 for index,run in enumerate(case["runs"]):
  if run["entry"]=="resource":continue
  command=[str(jdk/"bin/java.exe"),"-cp",runtime_cp,"intersect.oracle.ScalarEntryOracle",",".join(row["provider"]["registered_packages"]),grammar,run["entry"],run["text"]]
  r=subprocess.run(command,capture_output=True,text=True,encoding="utf-8",timeout=180)
  (out/(case["id"]+"-"+str(index)+".log")).write_text(r.stdout+r.stderr,encoding="utf-8",newline="\n");r.check_returncode()
  observations=[json.loads(line) for line in r.stdout.splitlines() if line.startswith("{")];assert len(observations)==1
  result={"case":case["id"],**observations[0]};results.append(result)
  print(json.dumps({k:v for k,v in result.items() if k!="entry_methods"}),flush=True)
(out/"observations.json").write_text(json.dumps(results,indent=2)+"\n",encoding="utf-8",newline="\n")
(out/"receipt.json").write_text(json.dumps({"plan_sha256":hashlib.sha256(plan_path.read_bytes()).hexdigest(),"adapter_sha256":hashlib.sha256(source.read_bytes()).hexdigest(),"requests":len(results),"skipped":skipped,"native_support":False},indent=2)+"\n",encoding="utf-8",newline="\n")
