"""Text-owned grammar registry and combined independent oracle inventory."""
import definition_ledger as model
from definition_ledger import *

XTEXT = "upstream/xtext/org/eclipse/xtext/Xtext.ecore"

GRAMMAR = "upstream/xtext/org/eclipse/xtext/Xtext.xtext"

TERMINALS = "upstream/xtext/org/eclipse/xtext/common/Terminals.xtext"

XTEXT_OWNERS = {
    "Grammar": "X02", "AbstractRule": "X07",
    "AbstractMetamodelDeclaration": "X02", "GeneratedMetamodel": "X03",
    "ReferencedMetamodel": "X02", "ParserRule": "X07", "TypeRef": "X02",
    "AbstractElement": "X11", "Action": "X09", "Keyword": "X06",
    "RuleCall": "X07", "Assignment": "X08", "CrossReference": "X14",
    "TerminalRule": "X04", "AbstractNegatedToken": "X05",
    "NegatedToken": "X05", "UntilToken": "X05", "Wildcard": "X05",
    "EnumRule": "X07", "EnumLiteralDeclaration": "X07",
    "Alternatives": "X11", "UnorderedGroup": "X13", "Group": "X11",
    "CharacterRange": "X04", "CompoundElement": "X11", "EOF": "X05",
    "Parameter": "X10", "NamedArgument": "X10", "Condition": "X10",
    "Conjunction": "X10", "Negation": "X10", "Disjunction": "X10",
    "CompositeCondition": "X10", "ParameterReference": "X10",
    "LiteralCondition": "X10", "Annotation": "X01",
}

OUTPUT = ROOT / "docs/definition-languages-ledger.json"

def obligation(*args, **kwargs):
    row = model.obligation(*args, **kwargs)
    if row['domain'] == 'xtext':
        row['owner'] = 'intersect-text::grammar'
    return row

def metamodel_obligations(root, path, domain, pins):
    rows = model.metamodel_obligations(root, path, domain, pins, owners=XTEXT_OWNERS)
    if domain == 'xtext':
        for row in rows:
            row['owner'] = 'intersect-text::grammar'
    return rows

def grammar_obligations(root, pins, path=GRAMMAR, prefix="syntax", milestone="X01"):
    source = (root / path).read_text(encoding="utf-8")
    # Fixed-input declaration scanner, not a general Xtext grammar parser.
    masked = re.sub(r"//[^\n]*|/\*[\s\S]*?\*/|'(?:\\.|[^'\\])*'|\"(?:\\.|[^\"\\])*\"",
                    lambda m: re.sub(r"[^\n]", " ", m[0]), source)
    pattern = r"(?m)^[ \t]*(?:(?:fragment|terminal)[ \t]+)*\^?(\w+)(?:\s+returns\s+\w+(?:::\w+)?)?\s*:"
    records = []
    for match in re.finditer(pattern, masked):
        name = match[1]
        line = source.count("\n", 0, match.start()) + 1
        records.append(obligation(
            f"xtext/{prefix}/{name}", "xtext", "grammar_rule", name, milestone,
            source_ref(path, f"line:{line}", pins),
            {"rule": name, "scope": "syntax recognition; execution has separate obligations"}))
    if not records:
        raise ValueError("No grammar rules inventoried")
    return records


def oracle_cases(root, pins):
    reference_path = "mercurio/pilot-controls.json"
    reference = read_json(root / reference_path)
    contract = read_json(root / "mercurio/preview-contract.json")
    if contract["reference_sha256"] != pins[reference_path]["sha256"]:
        raise ValueError("Preview contract pins a different oracle file")
    if reference["provenance"]["pilot_revision"] != contract["pilot_commit"]:
        raise ValueError("Pilot revision mismatch")
    controls, cases = reference["controls"], contract["cases"]
    if len(controls) != len(cases):
        raise ValueError("Oracle case inventory mismatch")
    rows, ids = [], set()
    for index, (control, case) in enumerate(zip(controls, cases)):
        if case["case_id"] in ids:
            raise ValueError("Duplicate oracle case identity")
        ids.add(case["case_id"])
        if digest(control["source"].encode("utf-8")) != case["source_sha256"]:
            raise ValueError(f"Oracle source mismatch: {case['case_id']}")
        expected = "core_constructed" if control["accepted"] else "syntax"
        if case["language"] != control["language"] or case["expected_stage"] != expected:
            raise ValueError(f"Oracle stage/language mismatch: {case['case_id']}")
        rows.append({**case, "source": source_ref(reference_path, f"/controls/{index}", pins),
                     "intersect_result": "not_run"})
    positive = sum(c["accepted"] for c in controls)
    nodes = sum(len(c["nodes"]) for c in controls)
    links = sum(len(c["links"]) for c in controls)
    expected_counts = (contract["positive_controls"], contract["syntax_negative_controls"],
                       contract["expected_native_nodes"], contract["expected_reference_slots"])
    if (positive, len(controls) - positive, nodes, links) != expected_counts:
        raise ValueError("Oracle node/reference/acceptance denominator changed")
    return {"cases": rows, "positive": positive, "syntax_negative": len(controls) - positive,
            "nodes": nodes, "reference_slots": links, "provenance": reference["provenance"],
            "scope": reference["scope"], "intersect_execution": "not_run",
            "semantic_validation": "not_assessed", "transformation_completion": "not_assessed"}


def build(root=FIXTURES):
    pins = verify_artifacts(root)
    extensions = read_json(root / "extension-obligations.json")
    rows = metamodel_obligations(root, ECORE, "ecore", pins)
    rows += metamodel_obligations(root, XTEXT, "xtext", pins)
    rows += grammar_obligations(root, pins)
    rows += grammar_obligations(root, pins, TERMINALS, "common", "X06")
    for entry in extensions["obligations"]:
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
        "obligations": rows, "portable_oracle": oracle_cases(root, pins),
        "limits": ["Case descriptions are plans; executable feature controls remain in their milestones.",
                   "Schema/syntax inventory cannot prove arbitrary behavior coverage; service obligations need continuing review.",
                   "Rcore v2 syntax is frozen in S01, not by this upstream language inventory.",
                   "Matching model bytes does not identify all shaded Java runtime dependencies."],
    }


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
