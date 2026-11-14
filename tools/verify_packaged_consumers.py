"""Build and run consumers of the actual .crate files, without workspace paths.

Nothing is published. A local patch supplies the unpublished companion artifact;
all other dependencies resolve from Cargo's normal offline cache. Keep the
extracted sources and logs under target for inspection.
"""
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile


def run(args, cwd):
    subprocess.run(args, cwd=cwd, check=True)


def extract(archive, destination):
    destination = destination.resolve()
    with tarfile.open(archive, "r:gz") as source:
        for member in source.getmembers():
            target = (destination / member.name).resolve()
            if not target.is_relative_to(destination):
                raise ValueError(f"Archive path escapes extraction root: {member.name}")
            if member.isdir():
                target.mkdir(parents=True, exist_ok=True)
            elif member.isfile():
                target.parent.mkdir(parents=True, exist_ok=True)
                stream = source.extractfile(member)
                if stream is None:
                    raise ValueError(f"Missing archive payload: {member.name}")
                with stream:
                    target.write_bytes(stream.read())
            else:
                raise ValueError(f"Unexpected archive member: {member.name}")


def main():
    root = Path(__file__).resolve().parent.parent
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1", "--offline"],
        cwd=root, text=True,
    ))
    versions = {p["name"]: p["version"] for p in metadata["packages"]}
    names = ("intersect-model", "intersect-text")
    run(["cargo", "package", "-p", names[0], "-p", names[1],
         "--allow-dirty", "--offline", "--no-verify"], root)
    output = Path(tempfile.mkdtemp(prefix="packaged-consumers-", dir=root / "target"))
    for name in names:
        extract(root / "target/package" / f"{name}-{versions[name]}.crate", output)
    model = output / f"intersect-model-{versions['intersect-model']}"
    xtext = output / f"intersect-text-{versions['intersect-text']}"
    consumer = output / "consumer"
    (consumer / "src").mkdir(parents=True)
    manifest = f'''[package]
name = "intersect-packaged-consumer"
default-run = "intersect-packaged-consumer"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
intersect-model = "={versions['intersect-model']}"
intersect-text = {{ version = "={versions['intersect-text']}", optional = true }}
serde_json = "1.0.149"

[features]
text = ["dep:intersect-text"]
xmi = ["intersect-model/xmi"]
sqlite = ["intersect-model/sqlite"]
codegen = ["intersect-model/codegen"]
rhai = ["intersect-model/rhai"]

[[bin]]
name = "packaged_rcore_model"
path = {json.dumps((model / 'examples/rcore_model.rs').as_posix())}

[[bin]]
name = "packaged_dynamic_library"
path = {json.dumps((model / 'examples/dynamic_library.rs').as_posix())}

[patch.crates-io]
intersect-model = {{ path = {json.dumps(model.as_posix())} }}
intersect-text = {{ path = {json.dumps(xtext.as_posix())} }}
'''
    (consumer / "Cargo.toml").write_text(manifest, encoding="utf-8")
    verifier = (root / "tests/model-only/src/lib.rs").read_text(encoding="utf-8")
    verifier = verifier.replace("../../../fixtures/rcore/api/organization.rcore", "organization.rcore")
    (consumer / "src/lib.rs").write_text(verifier, encoding="utf-8")
    (consumer / "src/organization.rcore").write_bytes(
        (root / "fixtures/rcore/api/organization.rcore").read_bytes())
    for fixture in ("library-project.rcore", "library.xtext", "catalog.library", "loans.library"):
        (consumer / "src" / fixture).write_bytes((root / "fixtures/library" / fixture).read_bytes())
    (consumer / "src/main.rs").write_text('''fn main() -> Result<(), String> {
    intersect_packaged_consumer::verify_boundary()?;
    #[cfg(feature = "text")]
    {
        let package = intersect_text::compiler::compile_rcore_project(
            include_str!("library-project.rcore"), include_str!("library.xtext"))
            .map_err(|e| format!("{e:?}"))?;
        let result = intersect_text::evaluate(&package, &[
            intersect_text::Source {uri:"memory:catalog.library".into(), text:include_str!("catalog.library").into()},
            intersect_text::Source {uri:"memory:loans.library".into(), text:include_str!("loans.library").into()},
        ]);
        if !result.diagnostics.is_empty() { return Err(format!("{:?}", result.diagnostics)); }
        if result.model.elements.is_empty() { return Err("Text consumer built no model".into()); }
    }
    println!("Packaged model lifecycle and enabled text consumer passed.");
    Ok(())
}
''', encoding="utf-8")
    run(["cargo", "run", "--offline"], consumer)
    run(["cargo", "run", "--offline", "--all-features"], consumer)
    run(["cargo", "check", "--offline", "--all-features", "--target", "wasm32-unknown-unknown"], consumer)
    for example in ("rcore_model", "dynamic_library"):
        run(["cargo", "run", "--offline", "--bin", f"packaged_{example}"], consumer)
    print(f"Packaged consumer evidence: {output}")


if __name__ == "__main__":
    main()
