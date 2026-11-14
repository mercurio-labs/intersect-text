"""Audit exact pinned KerML requirements from retained Xtext AST evidence."""
import argparse
from collections import Counter
import hashlib
import json
from pathlib import Path

EXTRACT = "crates/mercurio-sysml/resources/metamodels/sysml-2.0-pilot-2026-08/grammar.structure.extract.json"
CONTEXT = "org.omg.kerml.xtext.KerML"


def collect(mercurio, upstream):
    raw = (mercurio/EXTRACT).read_bytes()
    data = json.loads(raw)
    context = data["resolution_contexts"][CONTEXT]
    rules = {rule["id"]:rule for grammar in data["grammars"] for rule in grammar["rules"]}
    selected = set(context["effective_rules"].values())
    owners = {rule["id"]:grammar for grammar in data["grammars"] for rule in grammar["rules"]}
    sources = {}
    declarations = {}
    for identity in sorted(selected):
        grammar = owners[identity]
        source = grammar["source"]
        source_raw = (upstream/source["path"]).read_bytes()
        assert hashlib.sha256(source_raw).hexdigest() == source["sha256"], source["path"]
        sources[source["path"]] = dict(sha256=source["sha256"], bytes=len(source_raw))
        span = rules[identity]["span"]
        encoded = source_raw.decode("utf-8").encode("utf-16-le")
        start, end = span["offset"], span["offset"]+span["length"]
        assert 0 <= start < end <= len(encoded)//2
        exact = encoded[start*2:end*2].decode("utf-16-le").encode("utf-8")
        declarations[identity] = dict(kind=rules[identity]["kind"], source=source["path"], utf16_offset=start, utf16_length=end-start, utf8_slice_sha256=hashlib.sha256(exact).hexdigest())
    graph = {identity:set() for identity in selected}
    for site, target in context["rule_calls"].items():
        owner = site.split("/")[0]
        if owner in graph:
            assert target in rules, target
            graph[owner].add(target)
    closures = {}
    for name in ["FeatureSpecialization", "FeatureSpecializationPart", "PayloadFeatureSpecializationPart"]:
        start = context["effective_rules"][name]
        pending, seen = [start], set()
        while pending:
            node = pending.pop()
            if node in seen:
                continue
            seen.add(node)
            pending.extend(graph.get(node, ()))
        cycles = []
        def walk(node, path, done):
            if node in path:
                cycles.append(path[path.index(node):]+[node])
                return
            if node in done:
                return
            for target in sorted(graph.get(node, ())):
                walk(target, path+[node], done)
            done.add(node)
        walk(start, [], set())
        closures[name] = dict(rules=sorted(seen), cycles=cycles)
    kinds, flags, assignments, actions = Counter(), Counter(), Counter(), Counter()
    def inspect(value):
        if isinstance(value, dict):
            if "kind" in value:
                kind, fields = value["kind"], value.get("fields", {})
                kinds[kind] += 1
                for key in ["fragment", "firstSetPredicated", "predicated", "parameters", "arguments"]:
                    if fields.get(key):
                        flags[key] += 1
                if kind == "Assignment":
                    assignments[fields["operator"]] += 1
                if kind == "Action":
                    actions["simple" if not fields.get("feature") else fields["feature"]+fields["operator"]] += 1
            for child in value.values():
                inspect(child)
        elif isinstance(value, list):
            for child in value:
                inspect(child)
    for identity in sorted(selected):
        inspect(rules[identity])
    return dict(context=CONTEXT, extract=dict(path=EXTRACT, sha256=hashlib.sha256(raw).hexdigest(), provenance=data["provenance"], parser=data["parser"], span_encoding=data["span_encoding"]), sources=sources, effective_rule_count=len(selected), kinds=dict(sorted(kinds.items())), flags={key:flags[key] for key in ["fragment", "firstSetPredicated", "predicated", "parameters", "arguments"]}, assignments=dict(sorted(assignments.items())), actions=dict(sorted(actions.items())), first_set_predicates=context["first_set_predicates"], closures=closures, declarations=declarations, native_extraction=False, executable_support_awarded=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mercurio", type=Path, required=True)
    parser.add_argument("--upstream", type=Path, required=True)
    parser.add_argument("--emit", action="store_true")
    args = parser.parse_args()
    observed = collect(args.mercurio, args.upstream)
    if args.emit:
        print(json.dumps(observed, indent=2))
    else:
        expected = json.loads((Path(__file__).parent/"observations.json").read_bytes())
        assert observed == expected
        print("Verified 282 exact source-identified effective rules, construct inventory and acyclic predicate closures; no executable support awarded.")


if __name__ == "__main__":
    main()
