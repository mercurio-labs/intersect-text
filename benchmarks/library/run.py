"""Fresh-process Library scale measurements; Python 3.11+, Windows or Linux."""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import platform
import queue
import statistics
import subprocess
import threading
import time

ROOT = Path(__file__).resolve().parents[2]


def generate(path, count):
    if count < 1:
        raise ValueError("element count must include at least the Library root")
    with path.open("w", encoding="utf-8", newline="\n") as stream:
        stream.write("library Scale {\n")
        for i in range(1, count):
            stream.write(f"  member m{i};\n")
        stream.write("}\n")
    return hashlib.sha256(path.read_bytes()).hexdigest()


def memory(process):
    """OS resident bytes and lifetime high-water mark, not model heap size."""
    if os.name == "nt":
        from ctypes import wintypes

        class Counters(ctypes.Structure):
            _fields_ = [("cb", wintypes.DWORD), ("faults", wintypes.DWORD)] + [
                (name, ctypes.c_size_t) for name in
                ("peak", "rss", "pool_peak", "pool", "nonpaged_peak", "nonpaged", "pagefile", "pagefile_peak")
            ]

        counters = Counters()
        counters.cb = ctypes.sizeof(counters)
        api = ctypes.WinDLL("psapi", use_last_error=True).GetProcessMemoryInfo
        api.argtypes = [wintypes.HANDLE, ctypes.POINTER(Counters), wintypes.DWORD]
        api.restype = wintypes.BOOL
        if not api(wintypes.HANDLE(int(process._handle)), ctypes.byref(counters), counters.cb):
            raise ctypes.WinError(ctypes.get_last_error())
        return {"rss_bytes": counters.rss, "peak_rss_bytes": counters.peak}
    fields = {}
    for line in Path(f"/proc/{process.pid}/status").read_text().splitlines():
        if line.startswith(("VmRSS:", "VmHWM:")):
            key, value, _ = line.split()
            fields[key] = int(value) * 1024
    return {"rss_bytes": fields["VmRSS:"], "peak_rss_bytes": fields["VmHWM:"]}


def measure(command, timeout, log_path):
    events = queue.Queue()
    with log_path.open("w", encoding="utf-8") as log:
        process = subprocess.Popen(command, cwd=ROOT, stdin=subprocess.PIPE,
                                   stdout=subprocess.PIPE, stderr=log, text=True, bufsize=1)

        def read():
            for line in process.stdout:
                events.put(line.strip())
            events.put(None)

        reader = threading.Thread(target=read, daemon=True)
        reader.start()
        deadline = time.monotonic() + timeout
        samples = {}
        result = None
        try:
            while True:
                line = events.get(timeout=max(0.001, deadline - time.monotonic()))
                if line is None:
                    break
                if line in ("READY", "MEASURED"):
                    samples[line.lower()] = memory(process)
                    process.stdin.write("\n")
                    process.stdin.flush()
                elif line.startswith('{"engine"'):
                    result = json.loads(line)
                elif line.startswith("{"):
                    candidate = json.loads(line)
                    if "engine" in candidate:
                        result = candidate
                else:
                    log.write(line + "\n")
            code = process.wait(timeout=max(0.001, deadline - time.monotonic()))
            if code or result is None:
                result = {"status": "process_error", "exit_code": code}
        except (queue.Empty, subprocess.TimeoutExpired):
            result = {"status": "timeout"}
        finally:
            if process.poll() is None:
                process.kill()
            process.wait()
            reader.join(timeout=2)
            process.stdin.close()
            process.stdout.close()
        return {**result, "memory": samples, "command": command, "log": str(log_path)}


def version(command):
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=20, cwd=ROOT)
        return (result.stdout + result.stderr).strip()
    except (OSError, subprocess.TimeoutExpired) as error:
        return str(error)


def report(rows):
    lines = ["# Library scale observations", "",
             "Fresh processes; no JIT warmup. Times exclude startup, definition setup and file I/O.",
             "Intersect measures full evaluate; Xtext measures resource load. No speedup ratio is valid.",
             "Memory is process resident memory (includes runtime, input, model and temporary allocations),",
             "not retained model size. Peak is the OS lifetime high-water mark through verification.",
             "Rust result: " + ("canonical indexed store (no compatibility export)." if any(r.get("stage") == "evaluate_stored" for r in rows) else "owned compatibility model after export."), "",
             "| Elements | Engine | Status | Median successful ms | Median post RSS MiB | Median peak RSS MiB |",
             "| ---: | --- | --- | ---: | ---: | ---: |"]
    for count in sorted({r["requested_elements"] for r in rows}):
        for engine in ("intersect", "xtext"):
            group = [r for r in rows if r["requested_elements"] == count and r["engine"] == engine]
            if not group:
                continue
            successes = [r["elapsed_ms"] for r in group if r["status"] == "ok"]
            elapsed = f"{statistics.median(successes):.3f}" if successes else "—"
            values = []
            for key in ("rss_bytes", "peak_rss_bytes"):
                data = [r["memory"]["measured"][key] / 2**20 for r in group if "measured" in r.get("memory", {})]
                values.append(f"{statistics.median(data):.2f}" if data else "—")
            status = ", ".join(sorted({r["status"] for r in group}))
            lines.append(f"| {count:,} | {engine} | {status} | {elapsed} | {' | '.join(values)} |")
    lines += ["", "Rejected rows have no successful parse timing or completed-model memory measurement.",
              "See results.json for rejection latency, diagnostics, per-run measurements and provenance.", ""]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sizes", type=int, nargs="+", default=[100, 1000, 10000, 100000, 1000000])
    parser.add_argument("--repeats", type=int, default=3)
    parser.add_argument("--timeout", type=float, default=180)
    parser.add_argument("--java", default="java")
    parser.add_argument("--java-heap", default="4g")
    parser.add_argument("--rust-mode", choices=("export", "stored"), default="export",
                        help="Retain the owned compatibility model or canonical indexed store")
    parser.add_argument("--output", type=Path, default=ROOT / "target/library-benchmark")
    args = parser.parse_args()
    if args.repeats < 1 or args.timeout <= 0 or any(n < 1 for n in args.sizes):
        parser.error("sizes, repeats and timeout must be positive")
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    native = ROOT / "target/release" / ("library_benchmark.exe" if os.name == "nt" else "library_benchmark")
    native = native.parent / "examples" / native.name
    java_root = ROOT / "benchmarks/library/java"
    classpath = java_root / "target/classpath.txt"
    if not native.exists() or not classpath.exists():
        parser.error("build the Rust example and Java benchmark first (see README.md)")
    dependencies = classpath.read_text().strip().split(os.pathsep)
    metadata = {
        "platform": platform.platform(), "processor": platform.processor(), "logical_cpus": os.cpu_count(),
        "python": platform.python_version(), "rust": version(["rustc", "-Vv"]),
        "java": version([args.java, "-version"]),
        "git_revision": version(["git", "-c", "safe.directory=*", "rev-parse", "HEAD"]),
        "git_status": version(["git", "-c", "safe.directory=*", "status", "--short"]),
        "repeats": args.repeats, "timeout_seconds": args.timeout, "rust_mode": args.rust_mode,
        "dependencies": [{"name": Path(p).name, "sha256": hashlib.sha256(Path(p).read_bytes()).hexdigest()} for p in dependencies],
        "sources": {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest()
                    for folder in [ROOT / "fixtures/library", ROOT / "crates", ROOT / "benchmarks/library"]
                    for p in folder.rglob("*") if p.is_file() and p.suffix in (".rs", ".xtext", ".ecore", "-behavior.rcore", ".java", ".mwe2", ".py", ".xml")
                    and not any(part in ("target", "src-gen", "model") for part in p.relative_to(folder).parts)},
        "native_binary_sha256": hashlib.sha256(native.read_bytes()).hexdigest(),
    }
    rows = []
    for count in args.sizes:
        path = args.output / f"members-{count}.library"
        digest = generate(path, count)
        commands = {
            "intersect": [str(native), str(path), str(count)] + (["--stored"] if args.rust_mode == "stored" else []),
            "xtext": [args.java, "-Xms64m", f"-Xmx{args.java_heap}", "-cp",
                      os.pathsep.join([str(java_root / "target/classes"), *dependencies]),
                      "bench.Main", str(path), str(count)],
        }
        for repeat in range(args.repeats):
            # Alternate execution order to reduce systematic order bias.
            for engine in (list(commands) if repeat % 2 == 0 else list(reversed(commands))):
                print(f"{engine}: {count:,} elements, run {repeat + 1}", flush=True)
                row = measure(commands[engine], args.timeout, args.output / f"{engine}-{count}-{repeat}.log")
                rows.append({**row, "engine": engine, "requested_elements": count, "repeat": repeat,
                             "input_bytes": path.stat().st_size, "input_sha256": digest})
                (args.output / "results.json").write_text(json.dumps({"metadata": metadata, "runs": rows}, indent=2), encoding="utf-8")
                (args.output / "REPORT.md").write_text(report(rows), encoding="utf-8")
    print(args.output / "REPORT.md")


if __name__ == "__main__":
    main()
