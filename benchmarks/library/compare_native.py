"""Paired native release comparisons; optionally profile allocations separately."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import statistics

import run


def summary(rows):
    lines = ["# Native memory comparison", "", "Medians of fresh-process, uninstrumented full evaluations. RSS includes the process, not only live model objects.",
             "Retained result: " + ("canonical indexed store." if any(r.get("stage") == "evaluate_stored" for r in rows) else "owned compatibility model after export."), "",
             "| Elements | Build | Successful runs | Time ms | Peak RSS MiB | Post RSS MiB |",
             "| ---: | --- | ---: | ---: | ---: | ---: |"]
    for size in sorted({r["requested_elements"] for r in rows}):
        for build in ("baseline", "optimized"):
            selected = [r for r in rows if r["requested_elements"] == size and r["build"] == build and not r["instrumented"]]
            good = [r for r in selected if r["status"] == "ok" and r.get("verified")]
            numbers = [f"{statistics.median([r['elapsed_ms'] for r in good]):.3f}"] if good else ["—"]
            for key in ("peak_rss_bytes", "rss_bytes"):
                numbers.append(f"{statistics.median([r['memory']['measured'][key] / 2**20 for r in good]):.2f}" if good else "—")
            lines.append(f"| {size:,} | {build} | {len(good)}/{len(selected)} | {' | '.join(numbers)} |")
    return "\n".join(lines) + "\n"


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--baseline", type=Path, required=True)
    p.add_argument("--optimized", type=Path, required=True)
    p.add_argument("--profile-baseline", type=Path)
    p.add_argument("--profile-optimized", type=Path)
    p.add_argument("--sizes", type=int, nargs="+", default=[1000, 10000, 100000, 1000000])
    p.add_argument("--repeats", type=int, default=3)
    p.add_argument("--rust-mode", choices=("export", "stored"), default="export")
    p.add_argument("--timeout", type=float, default=180)
    p.add_argument("--output", type=Path, default=run.ROOT / "target/memory-comparison")
    a = p.parse_args()
    if min(a.sizes) < 1 or a.repeats < 1 or a.timeout <= 0:
        p.error("sizes, repeats and timeout must be positive")
    if bool(a.profile_baseline) != bool(a.profile_optimized):
        p.error("provide both profile binaries, or neither")
    binaries = {"baseline": a.baseline.resolve(), "optimized": a.optimized.resolve()}
    profiles = {"baseline": a.profile_baseline.resolve(), "optimized": a.profile_optimized.resolve()} if a.profile_baseline else {}
    for binary in [*binaries.values(), *profiles.values()]:
        if not binary.is_file():
            p.error(f"missing binary: {binary}")
    output = a.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    evidence = {"metadata": {
        "platform": platform.platform(), "processor": platform.processor(), "rust": run.version(["rustc", "-Vv"]),
        "base_revision": run.version(["git", "rev-parse", "HEAD"]),
        "working_diff_sha256": hashlib.sha256(run.version(["git", "diff"]).encode()).hexdigest(),
        "repeats": a.repeats, "timeout_seconds": a.timeout, "rust_mode": a.rust_mode,
        "binaries": {str(b): hashlib.sha256(b.read_bytes()).hexdigest() for b in [*binaries.values(), *profiles.values()]},
        "sources": {str(f.relative_to(run.ROOT)): hashlib.sha256(f.read_bytes()).hexdigest()
                    for f in (run.ROOT / "crates").rglob("*") if f.is_file() and f.suffix in (".rs", ".toml")},
    }, "runs": []}
    def trial(size, repeat, build, binary, instrumented):
        corpus = output / f"members-{size}.library"
        digest = run.generate(corpus, size)
        print(f"{build}: {size:,}, trial {repeat + 1}, allocation profile={instrumented}", flush=True)
        command = [str(binary), str(corpus), str(size)] + (["--stored"] if a.rust_mode == "stored" else [])
        result = run.measure(command, a.timeout,
                             output / f"{build}-{size}-{repeat}-{instrumented}.log")
        result.update(build=build, requested_elements=size, repeat=repeat, instrumented=instrumented,
                      input_sha256=digest, input_bytes=corpus.stat().st_size)
        evidence["runs"].append(result)
        (output / "results.json").write_text(json.dumps(evidence, indent=2), encoding="utf-8")
        (output / "REPORT.md").write_text(summary(evidence["runs"]), encoding="utf-8")
    for size in a.sizes:
        for repeat in range(a.repeats):
            for build in (["baseline", "optimized"] if repeat % 2 == 0 else ["optimized", "baseline"]):
                trial(size, repeat, build, binaries[build], False)
    for build, binary in profiles.items():
        trial(max(a.sizes), 0, build, binary, True)
    if any(r["status"] != "ok" or not r.get("verified") for r in evidence["runs"]):
        raise SystemExit("Some runs failed; inspect results.json")
    print(output / "REPORT.md")


if __name__ == "__main__":
    main()
