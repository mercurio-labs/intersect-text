"""Run fast-first Rust qualification and retain command/source evidence.

This runner covers the requested Rust lane, not full Rcore/Xtext acceptance.
Browser, independent oracle and per-feature runtime proofs remain separate.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import math
import os
import stat
import shutil
from pathlib import Path
import subprocess
import sys
import time


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=True) + "\n",
                    encoding="utf-8")


def physical_path(path):
    """Keep Windows long-path access idempotent; evidence retains normal paths."""
    if os.name != "nt":
        return path
    value = str(path.absolute())
    if value.startswith("\\\\?\\"):
        return Path(value)
    if value.startswith("\\\\"):
        return Path("\\\\?\\UNC\\" + value[2:])
    return Path("\\\\?\\" + value)


def pin(path):
    digest = hashlib.sha256()
    with physical_path(path).open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return {"path": str(path), "bytes": physical_path(path).stat().st_size,
            "sha256": digest.hexdigest()}


def git(root, *args):
    return subprocess.run(["git", "-C", str(root), *args], check=True,
                          stdout=subprocess.PIPE, stderr=subprocess.PIPE).stdout


def source_snapshot(root):
    # Include uncommitted and untracked source; ignored build/evidence outputs
    # are deliberately excluded. No checkout, file copy or index mutation.
    names = git(root, "ls-files", "--cached", "--others",
                "--exclude-standard", "-z").decode("utf-8").split("\0")
    files = {}
    for name in sorted(set(names) - {""}):
        path = physical_path(root / name)
        try:
            info = path.lstat()
        except FileNotFoundError:
            files[name] = {"kind": "missing"}
            continue
        # Path.is_file can hide OSError on some Python versions. Permission,
        # unsupported entry and other stat errors must reject the snapshot.
        if stat.S_ISLNK(info.st_mode):
            target = os.readlink(path)
            data = target.encode("utf-8")
            files[name] = {"kind": "symlink", "target": target,
                           "sha256": hashlib.sha256(data).hexdigest()}
        elif stat.S_ISREG(info.st_mode):
            item = pin(path)
            files[name] = {"kind": "file", "bytes": item["bytes"],
                           "sha256": item["sha256"]}
        else:
            raise ValueError(f"Unsupported source entry: {name}")
    return {"head": git(root, "rev-parse", "HEAD").decode("ascii").strip(),
            "files": files}


def source_difference(before, after):
    names = sorted(set(before["files"]) | set(after["files"]))
    return {"head_changed": before["head"] != after["head"],
            "changed_paths": [name for name in names
                              if before["files"].get(name) != after["files"].get(name)]}


SCOPED_PACKAGES = {
    "model": ["intersect-model"],
    "text": ["intersect-text"],
    "host": ["intersect-library", "intersect-web", "intersect-text-acceptance",
             "intersect-run", "intersect-project-host",
             "intersect-resource-lifecycle-tests", "intersect-model-text-tests"],
}


def make_plan(root, lane, toolchain, network, native_target, wasm_target,
              min_free_space_gib=None, scope="workspace"):
    configuration = root / 'repository-checks.json'
    local = json.loads(configuration.read_text(encoding='utf-8')) if configuration.is_file() else {}
    scoped_packages = local.get('scoped_packages', SCOPED_PACKAGES)
    if scope != 'workspace' and not scoped_packages.get(scope):
        raise ValueError(f'This repository does not own qualification scope {scope}')
    cargo = ["cargo"] + ([f"+{toolchain}"] if toolchain else [])
    flags = ["--locked"] + (["--offline"] if network == "offline" else [])
    selection = (["--workspace"] if scope == "workspace" else
                 [arg for name in scoped_packages[scope] for arg in ("-p", name)])
    formatter = [sys.executable, "-B", "tools/format_workspace.py", "--check"]
    if toolchain:
        formatter += ["--toolchain", toolchain]
    commands = [{"key": "format", "argv": formatter},
                {"key": "strict-all-target-clippy",
                 "argv": cargo + ["clippy", *selection, "--all-targets"]
                 + flags + ["--", "-D", "warnings"]}]

    def add(key, arguments, target=native_target):
        commands.append({"key": key, "argv": cargo + arguments + flags,
                         "target_dir": str(target)})

    def tests(key, arguments):
        commands.append({"key": key,
                         "argv": [sys.executable, "-B", "tools/test_report.py", "run", "--"]
                                 + arguments + flags,
                         "target_dir": str(native_target), "uses_cargo": True})

    commands[1]["target_dir"] = str(native_target)
    if scope != "workspace":
        commands.append({"key": "ownership-boundary",
                         "argv": [sys.executable, "-B", "tools/check_text_separation.py"]})
        commands.append({"key": "release-contract",
                         "argv": [sys.executable, "-B", "tools/check_text_release_contract.py"]})
        if scope == "model":
            commands.append({"key": "model-only-graph",
                             "argv": [sys.executable, "-B", "tools/check_model_boundary.py"]})
        if lane in ("native", "all"):
            tests(f"{scope}-quick-tests", ["--profile", "test-fast", *selection,
                                          "--lib", "--bins", "--tests"])
    elif lane in ("native", "all"):
        commands += [
            {"key": "definition-ledger",
             "argv": [sys.executable, "-B", "tools/" + local.get('definition_ledger', 'definition_ledger.py'), "--check"]},
            {"key": "portable-evidence",
             "argv": [sys.executable, "-B", "-m", "unittest", "discover",
                      "-s", "tools", "-p", "test_*.py"]},
        ]
        tests("workspace-tests", ["--workspace"])
        for control in local.get('feature_controls', [{"package": "intersect-semantic-bindings-tests", "features": "rhai"}]):
            tests(f"{control['package']}-{control['features']}-controls", ["-p", control['package'], "--features", control['features']])
        for package, example in local.get('examples', [
            ("intersect-model", "library"),
            ("intersect-text-acceptance", "model_api"),
            ("intersect-model", "dynamic_library"),
            ("intersect-model", "rcore_model"),
            ("intersect-rust-bindings-tests", "generated_library"),
        ]):
            add(f"example-{example}", ["run", "-p", package, "--example", example])
    if lane in ("wasm", "all"):
        add(f"{scope}-wasm-libraries",
            ["build", *selection, "--lib", "--target", "wasm32-unknown-unknown"],
            wasm_target)
        if scope in ("model", "text"):
            consumer = scope + "-only"
            add(f"{consumer}-wasm-consumer", ["build", "--manifest-path", f"tests/{consumer}/Cargo.toml",
                                            "--lib", "--target", "wasm32-unknown-unknown"], wasm_target)
            artifact = wasm_target / "wasm32-unknown-unknown/debug" / (
                f"intersect_{scope}_only_consumer.wasm")
            commands.append({"key": f"{scope}-wasm-execution",
                             "argv": ["node", f"tools/test_{scope}_boundary_wasm.mjs", str(artifact)]})
        elif scope == "host":
            for key, runner, artifact in local.get('host_wasm_controls', [
                ("resource-lifecycle", "test_resource_lifecycle_wasm.mjs", "intersect_resource_lifecycle_tests.wasm"),
                ("resource-interchange", "test_resource_interchange_wasm.mjs", "intersect_web.wasm"),
            ]):
                commands.append({"key": key + "-wasm-execution",
                                 "argv": ["node", "tools/" + runner,
                                          str(wasm_target / "wasm32-unknown-unknown/debug" / artifact)]})
    return {"schema_version": 1, "lane": lane, "root": str(root),
            "toolchain": toolchain, "network": network,
            "native_target_dir": str(native_target),
            "wasm_target_dir": str(wasm_target), "commands": commands,
            "minimum_free_space_bytes": (None if min_free_space_gib is None
                                         else math.ceil(min_free_space_gib * (1024 ** 3))),
            "package_scope": scope,
            "scope": "Selected-package Rust checks; no browser, oracle or full language gate award"}



BUILD_ENVIRONMENT = (
    "CARGO_INCREMENTAL", "CARGO_TARGET_DIR", "RUST_MIN_STACK", "RUSTFLAGS",
    "CARGO_PROFILE_DEV_OPT_LEVEL", "CARGO_PROFILE_DEV_DEBUG",
    "CARGO_PROFILE_TEST_OPT_LEVEL", "CARGO_PROFILE_TEST_DEBUG",
    "CARGO_PROFILE_RELEASE_OPT_LEVEL", "CARGO_PROFILE_RELEASE_DEBUG",
    "CARGO_PROFILE_DEV_STRIP", "CARGO_PROFILE_TEST_STRIP", "CARGO_PROFILE_RELEASE_STRIP",
)


def build_environment(environment):
    return {name: environment.get(name) for name in BUILD_ENVIRONMENT}


def tool_metadata(root, plan):
    versions = {}
    for executable in ("rustc", "cargo"):
        command = [executable] + ([f"+{plan['toolchain']}"]
                                 if plan.get("toolchain") else []) + ["--version"]
        result = subprocess.run(command, cwd=root, check=False,
                                stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        versions[executable] = {"argv": command, "exit_code": result.returncode,
                                "output": result.stdout.decode("utf-8", errors="replace")}
    if any(value["exit_code"] for value in versions.values()):
        raise RuntimeError(f"Toolchain version discovery failed: {versions}")
    return versions



def cache_volume_space(target_dir, minimum_free_bytes):
    # A target directory may not exist yet. Inspect its nearest existing parent
    # on the same resolved volume without creating or reserving cache space.
    requested = Path(target_dir).resolve()
    probe = requested
    while True:
        try:
            info = physical_path(probe).stat()
            if not stat.S_ISDIR(info.st_mode):
                probe = probe.parent
            break
        except FileNotFoundError:
            if probe == probe.parent:
                raise
            probe = probe.parent
    usage = shutil.disk_usage(physical_path(probe))
    return {"target_dir": str(requested), "probe_path": str(probe),
            "free_bytes": usage.free, "used_bytes": usage.used,
            "total_bytes": usage.total, "minimum_free_bytes": minimum_free_bytes,
            "threshold_pass": minimum_free_bytes is None or usage.free >= minimum_free_bytes}


def nonnegative_gib(value):
    parsed = float(value)
    if not math.isfinite(parsed) or parsed < 0 or parsed > (2 ** 63 - 1) / (1024 ** 3):
        raise argparse.ArgumentTypeError("Expected a finite non-negative GiB value within 64-bit byte range")
    return parsed


def run_plan(root, plan, output, snapshot=source_snapshot, run=subprocess.run,
             metadata=tool_metadata):
    output.mkdir(parents=True, exist_ok=False)
    write_json(output / "plan.json", plan)
    report = {"schema_version": 1, "state": "running",
              "started_utc": datetime.now(timezone.utc).isoformat(),
              "plan": pin(output / "plan.json"), "commands": [], "cache_observations": [],
              "source_postguard": {"state": "not_run"}, "gate_closures": []}
    before = None
    exit_code = 1
    try:
        report["runner"] = pin(Path(__file__).resolve())
        report["build_environment"] = build_environment(os.environ)
        before = snapshot(root)
        write_json(output / "source-before.json", before)
        report["source_before"] = pin(output / "source-before.json")
        report["toolchain_versions"] = metadata(root, plan)
        write_json(output / "progress.json", report)
        for index, command in enumerate(plan["commands"]):
            environment = os.environ.copy()
            environment["CARGO_NET_OFFLINE"] = str(plan["network"] == "offline").lower()
            if plan.get("toolchain"):
                environment["RUSTUP_TOOLCHAIN"] = plan["toolchain"]
            if command.get("target_dir"):
                environment["CARGO_TARGET_DIR"] = command["target_dir"]
            space = None
            if command.get("uses_cargo") or Path(command["argv"][0]).stem.lower() == "cargo":
                space = cache_volume_space(environment.get("CARGO_TARGET_DIR", root / "target"),
                                           plan.get("minimum_free_space_bytes"))
                report["cache_observations"].append({"command": command["key"], **space})
                write_json(output / "progress.json", report)
                if not space["threshold_pass"]:
                    report["error"] = {
                        "kind": "InsufficientCacheFreeSpace", "command": command["key"],
                        "message": "Configured minimum cache-volume free space was not met; command not started"}
                    write_json(output / "progress.json", report)
                    print(f"SKIP {command['key']}: free={space['free_bytes']} "
                          f"minimum={space['minimum_free_bytes']} bytes", flush=True)
                    break
            log = output / f"{index + 1:02d}-{command['key']}.log"
            print(f"START {command['key']}", flush=True)
            started = time.monotonic()
            with log.open("wb") as stream:
                result = run(command["argv"], cwd=root, env=environment,
                             stdout=stream, stderr=subprocess.STDOUT, check=False)
            item = {"key": command["key"], "argv": command["argv"],
                    "exit_code": result.returncode,
                    "seconds": time.monotonic() - started, "log": pin(log),
                    "build_environment": build_environment(environment)}
            if space is not None:
                item["cache_volume"] = space
            report["commands"].append(item)
            write_json(output / "progress.json", report)
            print(f"END {command['key']} exit={result.returncode}", flush=True)
            if result.returncode:
                exit_code = result.returncode if 0 < result.returncode < 256 else 1
                # Keep normal CI failure diagnostics visible as well as retaining
                # exact raw bytes in the downloadable run artifact.
                print(log.read_text(encoding="utf-8", errors="replace"), flush=True)
                break
        else:
            exit_code = 0
    except KeyboardInterrupt:
        exit_code = 130
        report["error"] = {"kind": "KeyboardInterrupt", "message": "Interrupted"}
    except Exception as error:
        report["error"] = {"kind": type(error).__name__, "message": str(error)}
    finally:
        if before is not None:
            try:
                after = snapshot(root)
                write_json(output / "source-after.json", after)
                difference = source_difference(before, after)
                unchanged = not difference["head_changed"] and not difference["changed_paths"]
                report["source_postguard"] = {
                    "state": "pass" if unchanged else "fail",
                    "source_after": pin(output / "source-after.json"),
                    "source_files": len(after["files"]), **difference}
                if not unchanged:
                    exit_code = exit_code or 1
            except Exception as error:
                report["source_postguard"] = {"state": "error",
                    "kind": type(error).__name__, "message": str(error)}
                exit_code = exit_code or 1
        report["state"] = "pass" if exit_code == 0 else "fail"
        report["exit_code"] = exit_code
        report["finished_utc"] = datetime.now(timezone.utc).isoformat()
        report["not_run"] = [c["key"] for c in plan["commands"][len(report["commands"]):]]
        write_json(output / "terminal.json", report)
        print(f"TERMINAL {report['state']}: {output / 'terminal.json'}", flush=True)
    return exit_code


def validate_output(root, output):
    if output.exists():
        raise ValueError("--output already exists; use a new run directory")
    ignored = subprocess.run(["git", "-C", str(root), "check-ignore", "--quiet",
                              "--", str(output)], check=False)
    if ignored.returncode != 0:
        raise ValueError("--output must be an ignored directory "
                         "(normally target/qualification/<run>)")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lane", choices=["preflight", "native", "wasm", "all"],
                        default="preflight")
    parser.add_argument("--toolchain", help="Optional installed Rust toolchain")
    parser.add_argument("--scope", choices=["workspace", "model", "text", "host"], default="workspace",
                        help="Select package ownership; workspace is milestone-only")
    parser.add_argument("--network", choices=["offline", "online"], default="offline")
    parser.add_argument("--native-target-dir", type=Path, default=Path("target"))
    parser.add_argument("--wasm-target-dir", type=Path,
                        help="Defaults to the native cache; use a separate path for parallel lanes")
    parser.add_argument("--min-free-space-gib", type=nonnegative_gib,
                        help="Optional minimum cache-volume free space before each Cargo command; not a sufficiency guarantee")
    parser.add_argument("--output", type=Path,
                        help="New ignored directory for immutable run evidence")
    parser.add_argument("--plan", action="store_true",
                        help="Print commands only; no Git, Cargo or output writes")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    native = (root / args.native_target_dir).resolve()
    wasm = (root / (args.wasm_target_dir or args.native_target_dir)).resolve()
    plan = make_plan(root, args.lane, args.toolchain, args.network, native, wasm,
                     args.min_free_space_gib, args.scope)
    if args.plan:
        print(json.dumps(plan, indent=2))
        return 0
    if args.output is None:
        parser.error("--output is required unless --plan is selected")
    output = (root / args.output).resolve()
    # A failed command must not dirty the source snapshot with its own evidence.
    try:
        validate_output(root, output)
    except ValueError as error:
        parser.error(str(error))
    return run_plan(root, plan, output)


if __name__ == "__main__":
    raise SystemExit(main())
