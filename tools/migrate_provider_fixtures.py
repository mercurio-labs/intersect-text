"""Regenerate Rcore provider fixtures through the native migration/compilation APIs.

Build intersect-web's rcore_behavior_parse example first. Original Ecore and
provider fixtures remain untouched. Rebound test snapshots are new fixture data,
not an automatic production model migration or fresh upstream oracle evidence.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
from migrate_behavior_fixtures import canonicalize_bundle_value

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = [
    "collections/collections", "feature-maps/features", "feature-references/references",
    "diagnostic-chains/diagnostics", "resource-handles/handles",
    "tree-iterators/iterators", "invocation-errors/errors",
]


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    binary = ROOT / "target/debug/examples" / ("rcore_behavior_parse.exe" if sys.platform == "win32" else "rcore_behavior_parse")

    def invoke(request):
        result = subprocess.run([str(binary)], input=json.dumps(request), text=True,
                                encoding="utf-8", capture_output=True, check=True, cwd=ROOT)
        value = json.loads(result.stdout)
        if not value.get("ok"):
            raise ValueError(value)
        return value

    def save(path, text):
        data = text.encode("utf-8")
        if args.check:
            if path.read_bytes() != data:
                raise ValueError(f"Stale migrated fixture: {path}")
        else:
            path.write_bytes(data)
        return digest(data)

    receipts = []
    for name in FIXTURES:
        original = ROOT / "fixtures" / (name + ".ecore")
        text = original.read_text(encoding="utf-8")
        report = invoke({"method": "ecore/migrate", "uri": "memory:ecore", "text": text,
                         "target_uri": "memory:model.rcore"})["report"]
        converted = report["converted"]
        old_id = report["original"]["id"]
        declaration_map = {json.dumps(row["original"], sort_keys=True): row["converted"]
                           for row in report["correspondence"]}
        identities = {}
        for row in report["correspondence"]:
            def identity(declaration):
                source = declaration["source"]
                return f'{source["uri"]}@{source["sha256"]}#' + "/".join(map(str, declaration["path"]))
            identities[identity(row["original"])] = identity(row["converted"])

        def remap(value):
            if isinstance(value, list):
                return [remap(item) for item in value]
            if isinstance(value, dict):
                if value == old_id:
                    return copy.deepcopy(converted["id"])
                key = json.dumps(value, sort_keys=True)
                if key in declaration_map:
                    return copy.deepcopy(declaration_map[key])
                if value.get("id") == old_id and value.get("kind") == "ecore":
                    result = copy.deepcopy(converted)
                    result["dependencies"] = remap(value["dependencies"])
                    return result
                return {key: remap(item) for key, item in value.items()}
            if isinstance(value, str):
                return identities.get(value, value)
            return value

        context_count = 0

        def rebuild_contexts(value):
            nonlocal context_count
            if isinstance(value, list):
                for item in value:
                    rebuild_contexts(item)
            elif isinstance(value, dict):
                for item in value.values():
                    rebuild_contexts(item)
                if {"bundle", "entry", "namespace", "model"}.issubset(value):
                    behavior = next(source["id"] for source in value["bundle"]["sources"]
                                    if source["kind"] == "rcore_behavior")
                    request = {"method": "compile_bundle", "compilation": {
                        "bundle": value["bundle"], "ecore": value["entry"], "namespace": value["namespace"],
                        "xtext": None, "rcore_behavior": behavior,
                        "materialized_types": value.get("materialized_types", []),
                    }}
                    for field in ["conversions", "applied_conversions", "constructors"]:
                        if field in value:
                            request[field] = value[field]
                    schema = invoke(request)["package"]["schema"]
                    value["model"]["schema_revision"] = schema["revision"]
                    context_count += 1

        output = original.with_name(original.stem + "-structure.rcore")
        converted_hash = save(output, converted["text"])
        contracts = original.parent / "conversions.json"
        receipt = {"original": original.relative_to(ROOT).as_posix(),
                   "original_sha256": digest(original.read_bytes()),
                   "converted": output.relative_to(ROOT).as_posix(), "converted_sha256": converted_hash,
                   "declarations": len(report["correspondence"]), "issues": report["issues"]}
        if contracts.exists():
            revised = canonicalize_bundle_value(remap(json.loads(contracts.read_text(encoding="utf-8"))))
            rebuild_contexts(revised)
            target = contracts.with_name("conversions-rcore.json")
            receipt.update({"contracts": contracts.relative_to(ROOT).as_posix(),
                            "contracts_sha256": digest(contracts.read_bytes()),
                            "converted_contracts": target.relative_to(ROOT).as_posix(),
                            "converted_contracts_sha256": save(target, json.dumps(revised, indent=2, ensure_ascii=False) + "\n"),
                            "rebound_test_snapshots": context_count})
        receipts.append(receipt)
    save(ROOT / "fixtures/rcore/provider-migration.json", json.dumps(receipts, indent=2, ensure_ascii=False) + "\n")
    print(f"{'Verified' if args.check else 'Generated'} {len(receipts)} migrated provider fixtures.")


if __name__ == "__main__":
    main()
