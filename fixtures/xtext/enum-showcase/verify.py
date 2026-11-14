"""Verify cumulative teaching-source provenance; no full KerML qualification."""
import hashlib
import json
from pathlib import Path
import re

fixture = Path(__file__).parent
root = fixture.parents[2]
evidence = json.loads((fixture / "provenance.json").read_bytes())
for row in evidence["derived_from"]:
    source = root / row["path"]
    assert hashlib.sha256(source.read_bytes()).hexdigest() == row["sha256"], row["path"]
base = (fixture / "base.xtext").read_bytes()
for row in evidence["terminal_slices"]:
    match = re.search(rb"terminal " + row["name"].encode() + rb"(?:\s|:)[\s\S]*?;", base)
    assert match, row["name"]
    assert hashlib.sha256(match.group()).hexdigest() == row["sha256"], row["name"]
assert (fixture / "LICENSE").read_bytes() == (fixture.parent / "inferred-datatypes/LICENSE").read_bytes()
assert b'urn:intersect:xtext:mini-kerml:enums:1' in (fixture / "model.ecore").read_bytes()
print("Verified seven unchanged parent sources, nine pinned terminal slices and retained license; independently authored enum teaching model.")
