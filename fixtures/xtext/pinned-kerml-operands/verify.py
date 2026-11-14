"""Integrity checks for imported operand evidence; does not execute Java."""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path
import zipfile


def digest(data):
    return hashlib.sha256(data).hexdigest()


def verify(mercurio, pilot):
    root = Path(__file__).resolve().parent
    provenance = json.loads((root / "provenance.json").read_text(encoding="utf-8"))
    for name, entry in provenance["imported_files"].items():
        assert digest((root / name).read_bytes()) == entry["sha256"], name
        assert (root / name).read_bytes() == (mercurio / entry["source"]).read_bytes(), name
    def git(root, *arguments):
        return subprocess.check_output(["git", "-C", str(root), *arguments])
    assert git(mercurio, "rev-parse", "HEAD").decode().strip() == provenance["mercurio_sysml_commit"]
    assert git(pilot, "rev-parse", "HEAD").decode().strip() == provenance["pilot_commit"]
    for name, expected in provenance["mercurio_source_sha256"].items():
        actual = (mercurio / name).read_bytes()
        assert digest(actual) == expected, name
        retained = git(mercurio, "show", provenance["mercurio_sysml_commit"] + ":" + name)
        assert actual.replace(b"\r\n", b"\n") == retained.replace(b"\r\n", b"\n"), name
    schema = provenance["pinned_schema"]
    assert digest((root / schema["path"]).read_bytes()) == schema["sha256"]
    assert (root / schema["path"]).read_bytes() == (pilot / schema["source"]).read_bytes()
    data = json.loads((root / "observations.json").read_text(encoding="utf-8"))
    original = data["provenance"]
    assert original["pilot_commit"] == provenance["pilot_commit"]
    for name, expected in original["source_sha256"].items():
        assert digest((pilot / name).read_bytes().replace(b"\r\n", b"\n")) == expected, name
    jar = pilot / "org.omg.sysml.interactive/target/org.omg.sysml.interactive-0.62.0-SNAPSHOT-all.jar"
    assert digest(jar.read_bytes()) == original["jar_sha256"]
    with zipfile.ZipFile(jar) as archive:
        for name, expected in original["class_sha256"].items():
            assert digest(archive.read("org/omg/sysml/" + name + ".class")) == expected, name
    for name, expected in provenance.get("additional_pilot_source_sha256", {}).items():
        actual = (pilot / name).read_bytes().replace(b"\r\n", b"\n")
        assert digest(actual) == expected
        assert actual == git(pilot, "show", provenance["pilot_commit"] + ":" + name).replace(b"\r\n", b"\n")
    cases = data["cases"]
    assert len(cases) == len({case["class"] for case in cases}) == 7
    for case in cases:
        assert case["operand_read_size"] == 0
        assert case["tree"]["kind"] == case["class"]
        relationships = case["tree"]["relationships"]
        assert len(relationships) == 2
        for membership in relationships:
            assert membership["kind"] == "ParameterMembership" and membership["visibility"] == "private"
            assert len(membership["elements"]) == 1
            parameter = membership["elements"][0]
            assert parameter["kind"] == "Feature" and parameter["direction"] == "in"
            assert len(parameter["relationships"]) == 1
            value = parameter["relationships"][0]
            assert value["kind"] == "FeatureValue" and value["visibility"] == "public"
            assert len(value["elements"]) == 1
            literal = value["elements"][0]
            assert literal["kind"] == "LiteralInteger" and literal["value"] == 2
    print("Seven imported operand controls: exact provenance and graph shape verified; Java not executed.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mercurio-sysml", type=Path, required=True)
    parser.add_argument("--pilot", type=Path, required=True)
    args = parser.parse_args()
    verify(args.mercurio_sysml, args.pilot)
