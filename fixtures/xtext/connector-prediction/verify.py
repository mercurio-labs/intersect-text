"""Verify the exact native negative-control graph and optional pinned sources."""
import argparse
import hashlib
import json
from pathlib import Path


def verify(upstream=None):
    fixture = Path(__file__).resolve().parent
    root = fixture.parents[2]
    pin = json.loads((fixture / "provenance.json").read_bytes())
    raw = (root / pin["program"]["path"]).read_bytes()
    assert hashlib.sha256(raw).hexdigest() == pin["program"]["sha256"]
    assert len(raw) == pin["program"]["bytes"]
    graph = json.loads(raw)
    for field in ["states", "rules", "decisions", "first_guards"]:
        assert len(graph[field]) == pin["program"][field], field
    assert graph["decisions"][pin["decision"]] == {
        "entries": pin["entries"], "exit": None
    }
    assert pin["native_extraction"] and not pin["executable_support_awarded"]
    if upstream is not None:
        for path, expected in pin["source_digests"].items():
            assert hashlib.sha256((upstream / path).read_bytes()).hexdigest() == expected, path
    return pin


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--upstream", type=Path)
    args = parser.parse_args()
    pin = verify(args.upstream)
    print(f"Verified {pin['program']['states']} native states and exact Connector replay identity; no runtime support awarded.")
