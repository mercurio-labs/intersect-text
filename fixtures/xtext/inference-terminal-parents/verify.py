"""Read-only replay of frozen terminal-library source inference observations."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import zipfile


def checked(path,digest):
    data=path.read_bytes()
    if hashlib.sha256(data).hexdigest()!=digest:raise ValueError(f"SHA-256 mismatch: {path}")
    return data


def validate_rows(cases,rows,imported):
    if [c["id"] for c in cases] != [r["id"] for r in rows] or len({c["id"] for c in cases})!=len(cases):
        raise ValueError("Terminal case order/identity differs")
    for case,row in zip(cases,rows):
        ids=[s["id"] for s in case["sources"]]
        names={s["grammar"]:s["source"] for s in case["sources"]}
        if len(set(ids))!=len(ids) or len(names)!=len(ids) or case["entry"]!=ids[-1] or row["entry"]!=case["entry"]:
            raise ValueError("Terminal source identities differ")
        for stage in ("initial","sources"):
            if [s["id"] for s in row[stage]]!=ids:raise ValueError("Source snapshot order differs")
            for authored,observed in zip(case["sources"],row[stage]):
                # Exact common.Terminals retains its license/header before grammar.
                if not authored["source"].isascii() or not re.search(r"(?m)^grammar " + re.escape(authored["grammar"]) + r"(?: |$)",authored["source"]):
                    raise ValueError("Missing exact authored grammar identity")
                if observed["grammar"]!=authored["grammar"]:raise ValueError("Declaring grammar differs")
                parents=[case["sources"][ids.index(x)]["grammar"] for x in authored["dependencies"]]
                if [p["name"] for p in observed["parents"]]!=parents:raise ValueError("Parent binding differs")
                errors=sum(d["severity"]=="ERROR" for d in observed["diagnostics"])
                if observed["error_count"]!=errors or observed["accepted"]!=(errors==0):raise ValueError("Source diagnostic outcome differs")
                for rule in observed["rules"]:
                    for item in [rule,*rule["calls"]]:
                        if not 0<=item["start"]<=item["end"]<=len(authored["source"]):raise ValueError("Declaration/call span exceeds exact source")
                    for call in rule["calls"]:
                        if observed["accepted"] and (call["proxy"] or call["target"]["grammar"] not in names):raise ValueError("Unresolved accepted call identity")
                def origins(value):
                    if isinstance(value,dict):
                        for origin in value.get("origins",[]):
                            text=names.get(origin["grammar"])
                            if text is None or not 0<=origin["start"]<=origin["end"]<=len(text):raise ValueError("Origin exceeds exact source")
                        for item in value.values():origins(item)
                    elif isinstance(value,list):
                        for item in value:origins(item)
                origins(observed["metamodels"])
        if row["accepted"]!=row["sources"][-1]["accepted"] or row["all_sources_accepted"]!=all(s["accepted"] for s in row["sources"]):
            raise ValueError("Whole-graph/entry acceptance differs")
        if row["imported_package"]!=imported or not row["imported_unchanged"]:raise ValueError("Original imported metadata changed")
        if [p["id"] for p in row["parent_snapshots"]]!=ids[:-1]:raise ValueError("Parent snapshot denominator differs")
        for before,after,observation in zip(row["initial"],row["sources"],row["parent_snapshots"]):
            if before["metamodels"]!=after["metamodels"] or not observation["metadata_unchanged"] or not observation["structure_unchanged"]:
                raise ValueError("Parent metamodel changed")
        rules={(s["grammar"],r["name"]):r for s in row["sources"] for r in s["rules"]}
        for source in row["sources"]:
            if not source["accepted"]:continue
            for rule in source["rules"]:
                for call in rule["calls"]:
                    target=call["target"];declared=rules.get((target["grammar"],target["name"]))
                    if declared is None or any(target[k]!=declared[k] for k in ("grammar","name","type","kind")):
                        raise ValueError("Exact rule target differs")


def frozen(fixture):
    provenance=json.loads((fixture/"provenance.json").read_text(encoding="utf-8"))
    if set(provenance["files"])!={"cases.json","observations.json","verify.py","README.md"}:raise ValueError("Fixture component denominator differs")
    for name,digest in provenance["files"].items():checked(fixture/name,digest)
    for name,digest in provenance["dependencies"].items():checked(fixture.parents[2]/name,digest)
    cases=json.loads((fixture/"cases.json").read_text(encoding="utf-8"));rows=json.loads((fixture/"observations.json").read_text(encoding="utf-8"))
    imported=json.loads((fixture.parent/"metamodel-inference/observations.json").read_text(encoding="utf-8"))[0]["imported_package"]
    validate_rows(cases,rows,imported)
    actual=(len(rows),sum(r["all_sources_accepted"] for r in rows),sum(r["accepted"] for r in rows),sum(len(r["sources"]) for r in rows))
    expected=(provenance["case_count"],provenance["complete_graph_accepted"],provenance["entry_only_accepted"],provenance["source_count"])
    if actual!=expected:raise ValueError("Terminal evidence denominator differs")
    if provenance["complete_graph_rejected"]!=len(rows)-actual[1] or provenance["parent_snapshots"]!=sum(len(r["parent_snapshots"]) for r in rows):raise ValueError("Graph/snapshot denominator differs")
    common=checked(fixture.parent/"terminal-parents/Terminals.xtext",provenance["common_terminals_sha256"])
    for case in cases:
        if case["sources"][0]["grammar"]=="org.eclipse.xtext.common.Terminals" and case["sources"][0]["source"].encode()!=common:
            raise ValueError("Pinned common.Terminals source was changed")
    return provenance,cases,rows


def replay(fixture,provenance,cases,jdk,cache,output):
    output=output.resolve();protected=fixture.parents[1]
    if output==protected or protected in output.parents:raise ValueError("Oracle output must stay outside fixtures")
    jars=[]
    for item in provenance["jars"]:
        path=cache/item["path"];checked(path,item["sha256"]);jars.append(str(path))
    archive=cache/provenance["source_archive"]["path"];checked(archive,provenance["source_archive"]["sha256"])
    with zipfile.ZipFile(archive) as source:
        if hashlib.sha256(source.read("org/eclipse/xtext/common/Terminals.xtext")).hexdigest()!=provenance["common_terminals_sha256"]:raise ValueError("Upstream terminal member differs")
        for item in provenance["source_members"]:
            if hashlib.sha256(source.read(item["path"])).hexdigest()!=item["sha256"]:raise ValueError("Upstream inference member differs")
    lock=json.loads(checked(fixture.parents[2]/"fixtures/definition-conformance/upstream/artifacts.json",provenance["upstream_lock_sha256"]))
    locked={item.get("archive_sha256") for item in lock["files"]}
    if provenance["source_archive"]["sha256"] not in locked or provenance["jars"][0]["sha256"] not in locked:raise ValueError("Upstream archive lock differs")
    output.mkdir(parents=True,exist_ok=True)
    for case in cases:
        if not re.fullmatch(r"[a-z][a-z0-9_]*",case["id"]):raise ValueError("Invalid artifact case identity")
        directory=output/case["id"];directory.mkdir(exist_ok=True)
        for source in case["sources"]:
            if not re.fullmatch(r"[a-z][a-z0-9_]*",source["id"]):raise ValueError("Invalid artifact source identity")
            (directory/(source["id"]+".xtext")).write_text(source["source"],encoding="utf-8",newline="\n")
        (directory/"order.txt").write_text("\n".join(s["id"] for s in case["sources"])+"\n",encoding="utf-8",newline="\n")
        (directory/"entry.txt").write_text(case["entry"],encoding="utf-8")
    (output/"imported.ecore").write_bytes((fixture.parent/"metamodel-inference/imported.ecore").read_bytes())
    suffix=".exe" if os.name=="nt" else "";cp=os.pathsep.join(jars)
    subprocess.run([str(jdk/"bin"/("javac"+suffix)),"-encoding","UTF-8","-cp",cp,"-d",str(output),str(fixture.parent/"metamodel-inference/MetamodelOracle.java"),str(fixture.parent/"inference-composition/CompositionOracle.java")],check=True,timeout=60)
    result=subprocess.run([str(jdk/"bin"/("java"+suffix)),"-cp",str(output)+os.pathsep+cp,"intersect.oracle.CompositionOracle",str(output),*[c["id"] for c in cases]],check=True,capture_output=True,text=True,encoding="utf-8",timeout=120)
    (output/"oracle-stderr.log").write_text(result.stderr,encoding="utf-8",newline="\n")
    rows=[json.loads(line) for line in result.stdout.splitlines() if line.startswith("{")]
    imported=json.loads((fixture.parent/"metamodel-inference/observations.json").read_text(encoding="utf-8"))[0]["imported_package"]
    validate_rows(cases,rows,imported)
    (output/"observations-replay.json").write_text(json.dumps(rows,indent=2)+"\n",encoding="utf-8",newline="\n")
    return rows


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jdk",type=Path,required=True);parser.add_argument("--artifact-dir",type=Path,required=True)
    parser.add_argument("--maven-cache",type=Path,default=Path.home()/".m2/repository");args=parser.parse_args()
    fixture=Path(__file__).resolve().parent;p,cases,expected=frozen(fixture)
    if replay(fixture,p,cases,args.jdk,args.maven_cache,args.artifact_dir)!=expected:raise ValueError("Complete terminal-parent observations differ")
    print(f"Verified {len(expected)} complete terminal-library graphs; native support not awarded by oracle extraction.")


if __name__=="__main__":main()
