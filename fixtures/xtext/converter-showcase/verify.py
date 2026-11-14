"""Check cumulative converter teaching sources without rewriting their identities."""
from pathlib import Path
import hashlib
import json
import re
f = Path(__file__).parent
root = f.parents[2]
e = json.loads((f / "provenance.json").read_bytes())
for row in e["derived_from"]:
    assert hashlib.sha256((root / row["path"]).read_bytes()).hexdigest() == row["sha256"], row["path"]
base = (f / "base.xtext").read_bytes()
for row in e["terminal_slices"]:
    match = re.search(rb"terminal " + row["name"].encode() + rb"(?:\s|:)[\s\S]*?;", base)
    assert match, row["name"]
    assert hashlib.sha256(match.group()).hexdigest() == row["sha256"], row["name"]
for row in e["converter_rules"]:
    raw = row["text"].encode("utf-8")
    assert base.count(raw) == 1
    assert hashlib.sha256(raw).hexdigest() == row["sha256"]
policy = json.loads((f / "model-behavior.rcore").read_bytes())["rule_conversions"][0]
assert policy["context"] == "mini.ConverterShowcase"
assert policy["declaration"]["source"] == {"uri": "base.xtext", "sha256": hashlib.sha256(base).hexdigest()}
assert base[policy["declaration"]["start"]:policy["declaration"]["end"]] == e["converter_rules"][0]["text"].encode("utf-8")
assert policy["keywords"] == {"nonunique": False}
assert (f / "LICENSE").read_bytes() == (f.parent / "enum-showcase/LICENSE").read_bytes()
assert (f / "UPSTREAM-NOTICE.txt").read_bytes() == (f.parent / "rule-converters/UPSTREAM-NOTICE.txt").read_bytes()
assert b"urn:intersect:xtext:mini-kerml:converters:1" in (f / "model.ecore").read_bytes()
print("Verified seven unchanged parent files, nine exact terminals, two exact datatype rules, converter binding source/span and retained licenses. This remains an authored KerML teaching schema.")
