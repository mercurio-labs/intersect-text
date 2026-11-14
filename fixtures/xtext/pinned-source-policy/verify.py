"""Verify retained pinned bytes under explicit source policy; no schema/runtime claim."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--upstream",type=Path,required=True)
    parser.add_argument("--inspector",type=Path,required=True)
    parser.add_argument("--resolve", action="store_true", help="Also verify complete retained resolver inventory")
    args=parser.parse_args()
    expected=json.loads((Path(__file__).parent/"observations.json").read_text(encoding="utf-8"))
    source=args.upstream/expected["source_path"]
    # Use the same slash-form source identifier on Windows and POSIX; never rewrite bytes.
    resource_uri=source.as_posix()
    raw=source.read_bytes()
    assert len(raw)==expected["bytes"]
    assert hashlib.sha256(raw).hexdigest()==expected["sha256"]
    for limit in [None,expected["bytes"]-1,expected["bytes"]]:
        command=[str(args.inspector),resource_uri]
        if limit is not None:
            command.append(str(limit))
        result=subprocess.run(command,text=True,capture_output=True,timeout=60)
        if limit!=expected["bytes"]:
            assert result.returncode!=0,result.stdout
            assert expected["default_result"] in result.stderr,result.stderr
        else:
            assert result.returncode==0,result.stderr
            observed=json.loads(result.stdout)
            for key in ["bytes","classifiers","root","semantically_verified"]:
                assert observed[key]==expected[key],(key,observed)
            assert observed["source"]=={"uri":resource_uri,"sha256":expected["sha256"]}
    if args.resolve:
        result=subprocess.run([str(args.inspector),resource_uri,str(expected["bytes"]),"--resolve"],text=True,capture_output=True,timeout=60)
        assert result.returncode==0,result.stderr
        observed=json.loads(result.stdout)
        assert observed["source"]=={"uri":resource_uri,"sha256":expected["sha256"]}
        assert observed["resolved_inventory"]=={
            "sources":[observed["source"]], "generic_types":0, "parameters":0,
            "raw_signatures":787, "retained_payloads":1674, "executable_schema":False,
        },observed
        assert observed["semantically_verified"] is False
        print("Verified resolver inventory: 787 raw signatures and 1674 retained payloads; not executable semantics.")
    assert source.read_bytes()==raw
    print("Verified default and one-byte-short rejection; exact policy retains pinned source and 182 classifier nodes. No semantic qualification.")


if __name__=="__main__":
    main()
