"""Model-owned structural inventory engine; no Text tooling dependency."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import sys
import xml.etree.ElementTree as ET

ROOT = Path(__file__).resolve().parents[1]

FIXTURES = ROOT / "fixtures/definition-conformance"

XSI = "{http://www.w3.org/2001/XMLSchema-instance}type"

ECORE = "upstream/ecore/model/Ecore.ecore"

ECORE_OWNERS = {
    "EAttribute": "E09", "EAnnotation": "E12", "EClass": "E03",
    "EClassifier": "E04", "EDataType": "E06", "EEnum": "E05",
    "EEnumLiteral": "E05", "EFactory": "E11", "EModelElement": "E12",
    "ENamedElement": "E02", "EObject": "E11", "EOperation": "E12",
    "EPackage": "E02", "EParameter": "E12", "EReference": "E10",
    "EStructuralFeature": "E09", "ETypedElement": "E09",
    "EStringToStringMapEntry": "E13", "EGenericType": "E07",
    "ETypeParameter": "E07",
}

OUTPUT = ROOT / "docs/model-definition-ledger.json"

def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def local_path(root, value):
    path = PurePosixPath(value)
    if path.is_absolute() or ".." in path.parts or "\\" in value or ":" in value:
        raise ValueError(f"Nonportable fixture path: {value}")
    result = root.joinpath(*path.parts).resolve()
    if not result.is_relative_to(root.resolve()):
        raise ValueError(f"Fixture escapes root: {value}")
    return result


def verify_artifacts(root):
    files = {}
    for name in ["upstream/artifacts.json", "mercurio/artifacts.json"]:
        lock = read_json(root / name)
        if lock.get("schema_version") != 1:
            raise ValueError(f"Unsupported artifact manifest: {name}")
        for item in lock["files"]:
            path = item["path"]
            if path in files:
                raise ValueError(f"Duplicate pinned artifact: {path}")
            if digest(local_path(root, path).read_bytes()) != item["sha256"]:
                raise ValueError(f"Artifact hash mismatch: {path}")
            files[path] = item
    return files


def source_ref(path, selector, pins):
    if path not in pins:
        raise ValueError(f"Unpinned source: {path}")
    return {"path": path, "sha256": pins[path]["sha256"], "selector": selector}


def obligation(identity, domain, kind, title, milestone, source, contract, cases=None):
    return {
        "id": identity, "domain": domain, "kind": kind, "title": title,
        "milestone": milestone,
        "owner": "intersect-model::definitions" if kind in ["grammar_rule", "classifier", "constraint"]
        else "intersect-model" if domain == "ecore"
        else "intersect-model",
        "source": source, "contract": contract,
        "planned_cases": cases or {
            "positive": f"Preserve and execute {title} in a valid context; compare independent observation.",
            "negative": f"Reject invalid type/reference/applicability for {title} at the prescribed stage.",
            "boundary": f"Exercise absent/default, inherited and collection/overload boundaries for {title}; retain identity after reload.",
        },
        "case_status": "planned_not_implemented",
        "imported": "not_assessed", "consumed": "not_assessed",
        "semantically_verified": "not_assessed",
        "implementation_evidence": [], "verification_evidence": [],
        "dependencies": ["native_import", "resolved_identity", "native_consumer", "independent_controls"],
    }


def metamodel_obligations(root, path, domain, pins, owners=None):
    package = ET.parse(root / path).getroot()
    if any(child.tag != "eClassifiers" for child in package):
        raise ValueError(f"Uninventoried package child in {path}")
    result = [obligation(f"{domain}/package", domain, "package", package.attrib["name"],
                         "E02" if domain == "ecore" else "X02",
                         source_ref(path, "/EPackage", pins), dict(package.attrib))]
    owners = ECORE_OWNERS if domain == "ecore" else owners
    if owners is None:
        raise ValueError(f"Missing structural inventory registry: {domain}")
    seen = set()
    for index, classifier in enumerate(package.findall("eClassifiers")):
        name = classifier.attrib["name"]
        if name in seen:
            raise ValueError(f"Duplicate classifier: {domain}/{name}")
        seen.add(name)
        kind = classifier.attrib[XSI].split(":")[-1]
        if name not in owners and not (domain == "ecore" and kind == "EDataType"):
            raise ValueError(f"Unowned upstream classifier: {domain}/{name}")
        milestone = owners.get(name, "E06")
        base = f"{domain}/{name}"
        selector = f"/EPackage/eClassifiers[{index + 1}]"
        result.append(obligation(base, domain, "classifier", name, milestone,
                                 source_ref(path, selector, pins), dict(classifier.attrib)))

        def walk(node, parent_id, parent_selector):
            offsets = Counter()
            for child in node:
                offsets[child.tag] += 1
                slot = offsets[child.tag]
                child_selector = f"{parent_selector}/{child.tag}[{slot}]"
                label = child.get("name", child.tag)
                identity = f"{parent_id}/{child.tag}/{label}/{slot}"
                # Source selectors retain overloads, parameters, generics and
                # annotation nodes instead of collapsing declarations by name.
                if child.tag == "details" and child.get("key") == "constraints":
                    for constraint in child.get("value", "").split():
                        result.append(obligation(
                            identity + "/" + constraint, domain, "constraint",
                            f"{name}.{constraint}", milestone,
                            source_ref(path, child_selector, pins), dict(child.attrib)))
                else:
                    result.append(obligation(
                        identity, domain, child.tag, f"{name}.{label}", milestone,
                        source_ref(path, child_selector, pins), dict(child.attrib)))
                walk(child, identity, child_selector)

        walk(classifier, base, selector)
    return result


def build(root=FIXTURES):
    pins = verify_artifacts(root)
    extensions = read_json(root / "extension-obligations.json")
    rows = metamodel_obligations(root, ECORE, "ecore", pins)
    for entry in extensions["obligations"]:
        if entry["domain"] != "ecore":
            continue
        if entry["selector"] not in local_path(root, entry["source_path"]).read_text(encoding="utf-8"):
            raise ValueError(f"Missing service source selector: {entry['id']}")
        rows.append(obligation(entry["id"], entry["domain"], "service", entry["title"],
                               entry["milestone"], source_ref(entry["source_path"], entry["selector"], pins),
                               {"requirement": entry["requirement"]}, entry["cases"]))
    ids = [r["id"] for r in rows]
    if len(ids) != len(set(ids)):
        raise ValueError("Duplicate obligation identity")
    for row in rows:
        if set(row["planned_cases"]) != {"positive", "negative", "boundary"} or not all(row["planned_cases"].values()):
            raise ValueError(f"Missing case plan: {row['id']}")
        if not re.fullmatch(r"[MEXSIQ]\d{2}", row["milestone"]):
            raise ValueError(f"Invalid milestone: {row['id']}")
    rows.sort(key=lambda row: row["id"])
    return {
        "schema_version": 1,
        "generated_by": "tools/definition_ledger.py; edit inputs and regenerate",
        "scope": "Pinned metamodel/syntax/service inventory, not native language support",
        "pins": read_json(root / "upstream/artifacts.json")["pins"],
        "inputs": {p: digest((root / p).read_bytes()) for p in [
            "upstream/artifacts.json", "mercurio/artifacts.json", "extension-obligations.json"]},
        "counts": {"obligations": len(rows), "by_domain": dict(sorted(Counter(r["domain"] for r in rows).items())),
                   "by_kind": dict(sorted(Counter(r["kind"] for r in rows).items()))},
        "obligations": rows, "portable_oracle": {"scope": "Ecore-only inventory; combined language oracle is owned by Text", "cases": [], "intersect_execution": "not_run"},
        "limits": ["Case descriptions are plans; executable feature controls remain in their milestones.",
                   "Schema/syntax inventory cannot prove arbitrary behavior coverage; service obligations need continuing review.",
                   "Rcore v2 syntax is frozen in S01, not by this upstream language inventory.",
                   "Matching model bytes does not identify all shaded Java runtime dependencies."],
    }


def encoded(value):
    return json.dumps(value, indent=2, ensure_ascii=False) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true", help="Check generated ledger without writing")
    args = parser.parse_args()
    try:
        result = build()
        text = encoded(result)
        if args.check:
            if OUTPUT.read_text(encoding="utf-8") != text:
                raise ValueError("Ledger differs; run python -B tools/definition_ledger.py")
        else:
            OUTPUT.write_text(text, encoding="utf-8", newline="\n")
        print(json.dumps({"status": "verified" if args.check else "generated", **result["counts"],
                          "oracle_cases": len(result["portable_oracle"]["cases"]), "runtime_support_awarded": False}))
        return 0
    except (ValueError, KeyError, OSError, ET.ParseError) as error:
        print(f"Definition ledger error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
