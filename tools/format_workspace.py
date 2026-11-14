"""Format every workspace Rust target without exceeding Windows command limits."""
import argparse
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--toolchain", help="Optional installed Rust toolchain")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    cargo = ["cargo"] + ([f"+{args.toolchain}"] if args.toolchain else [])
    result = subprocess.run(
        cargo + ["metadata", "--no-deps", "--format-version", "1", "--locked", "--offline"],
        cwd=root, check=True, capture_output=True, text=True, encoding="utf-8",
    )
    metadata = json.loads(result.stdout)
    members = set(metadata["workspace_members"])
    by_edition = {}
    for package in metadata["packages"]:
        if package["id"] in members:
            for target in package["targets"]:
                if Path(target["src_path"]).suffix == ".rs":
                    by_edition.setdefault(target["edition"], set()).add(target["src_path"])
    formatter = (["rustup", "run", args.toolchain, "rustfmt"]
                 if args.toolchain else ["rustfmt"])
    failed = False
    count = 0
    for edition, paths in sorted(by_edition.items()):
        paths = sorted(paths)
        count += len(paths)
        # Each root includes its modules, just as cargo fmt does. Small batches
        # avoid the Windows process command-line limit for our acceptance crate.
        for start in range(0, len(paths), 20):
            command = formatter + ["--edition", edition]
            if args.check:
                command.append("--check")
            completed = subprocess.run(command + paths[start:start + 20], cwd=root)
            failed |= completed.returncode != 0
    print(f"{'Checked' if args.check else 'Formatted'} {count} workspace Rust targets.")
    raise SystemExit(1 if failed else 0)


if __name__ == "__main__":
    main()
