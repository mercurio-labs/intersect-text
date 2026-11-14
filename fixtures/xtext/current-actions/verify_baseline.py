"""Check the frozen NativeV4 baseline before assigned-current support is added."""
import argparse
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native", type=Path, required=True)
    args = parser.parse_args()
    fixture = Path(__file__).resolve().parent
    request = {"method": "compile", "xtext_profile": "intersect.xtext.native.v4"}
    for kind, name in [("ecore", "model.ecore"), ("xtext", "model.xtext"), ("rcore_behavior", "model-behavior.rcore")]:
        request[kind] = (fixture / name).read_text(encoding="utf8")

    def invoke(value):
        result = subprocess.run([str(args.native)], input=json.dumps(value).encode(), capture_output=True, check=True, timeout=60)
        return json.loads(result.stdout)

    rejected = invoke(request)
    if rejected.get("ok") or not any("Assigned-current" in d["message"] for d in rejected.get("diagnostics", [])):
        raise ValueError(f"Unexpected frozen assigned-current baseline: {rejected.get('diagnostics')}")
    original = "Literal ({k::OperatorExpression.operand += current} operator=AdditiveOperator operand+=Literal)*"
    control = request["xtext"].replace(original, "{k::OperatorExpression} operator=AdditiveOperator operand+=Literal")
    if control == request["xtext"]:
        raise ValueError("Controlled grammar replacement did not match")
    compiled = invoke({**request, "xtext": control})
    if not compiled.get("ok"):
        raise ValueError(f"Existing simple-action control failed: {compiled.get('diagnostics')}")
    evaluated = invoke({"method": "evaluate", "package": compiled["package"], "sources": [{"uri": "control.kerml", "text": "Demo = + 1;"}]})
    if evaluated["diagnostics"] or len(evaluated["model"]["elements"]) != 3:
        raise ValueError("Existing simple-action execution control failed")
    derived = request["ecore"].replace('name="operand" containment="true"', 'name="operand" derived="true" transient="true" volatile="true" containment="true"')
    protected = invoke({**request, "xtext": control, "ecore": derived})
    if protected.get("ok") or not any("Cannot assign derived feature" in d["message"] for d in protected.get("diagnostics", [])):
        raise ValueError("Derived-feature write protection changed")
    print("NativeV4: assigned-current rejects; simple-action control executes three objects; derived operand assignment rejects.")


if __name__ == "__main__":
    main()
