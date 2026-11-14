"""Handwritten negative controls for source ownership and reverse dependencies."""
import tempfile
from pathlib import Path
import unittest

import check_text_separation as guard


class SeparationGuardTests(unittest.TestCase):
    def test_each_repository_checks_its_owned_package_without_a_sibling(self):
        for owner in ('model', 'text'):
            with self.subTest(owner=owner), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / 'Cargo.toml').write_text('[workspace]\n')
                package = root / f'crates/intersect-{owner}'
                (package / 'src').mkdir(parents=True)
                manifest = package / 'Cargo.toml'
                manifest.write_text(f'[package]\nname="intersect-{owner}"\n')
                self.assertEqual(guard.check_ownership(root), [])
                forbidden = 'intersect-text' if owner == 'model' else 'intersect-web'
                manifest.write_text(manifest.read_text() + f'\n[dev-dependencies]\nblocked={{package="{forbidden}",version="0.1"}}\n')
                self.assertTrue(guard.check_ownership(root))
    def test_new_source_coupling_rejects(self):
        actual = {"new.rs": {"grammar_reference": 1}}
        self.assertTrue(guard.source_errors(actual))

    def test_every_text_coupling_is_rejected_without_a_budget(self):
        for count in [1, 3]:
            self.assertTrue(guard.source_errors({"old.rs": {"grammar_reference": count}}))
        self.assertEqual(guard.source_errors({}), [])

    def test_legacy_allowlist_cannot_grant_an_exception(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text("[workspace]\n")
            for owner in ["model", "text"]:
                package = root / f"crates/intersect-{owner}"
                (package / "src").mkdir(parents=True)
                (package / "Cargo.toml").write_text(f'[package]\nname="intersect-{owner}"\n')
            (root / "crates/intersect-model/src/hidden.rs").write_text("pub struct Grammar;\n")
            (root / "tools").mkdir()
            (root / "tools/xtext_separation_allowlist.json").write_text(
                '{"entries":{"crates/intersect-model/src/hidden.rs":{"counts":{"typed_grammar":999}}}}')
            self.assertTrue(guard.check_ownership(root))

    def test_all_dependency_kinds_and_aliases_reject(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text('[workspace.dependencies]\ntext = { package = "intersect-text", version = "0.1" }\n', encoding="utf-8")
            for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
                with self.subTest(kind=kind):
                    manifest = {kind: {"text": {"workspace": True, "optional": True}}}
                    self.assertTrue(guard.dependency_errors(root, manifest))
                    manifest = {"target": {"cfg(target_arch = \"wasm32\")": {kind: {"alias": {"package": "intersect-text"}}}}}
                    self.assertTrue(guard.dependency_errors(root, manifest))

    def test_path_alias_rejects(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text('[workspace]\n', encoding="utf-8")
            dependency = root / "crates/hidden"
            dependency.mkdir(parents=True)
            (dependency / "Cargo.toml").write_text('[package]\nname = "intersect-text"\n', encoding="utf-8")
            errors = guard.dependency_errors(root, {"dependencies": {"alias": {"path": "../hidden"}}})
            self.assertTrue(errors)

    def test_generic_xtext_artifact_kind_is_retained_data(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "crates/intersect-model/src/definitions"
            source.mkdir(parents=True)
            (source / "source.rs").write_text('pub enum SourceKind { Rcore, Xtext }\n', encoding="utf-8")
            self.assertEqual(guard.source_counts(root), {})
            (source / "new.rs").write_text('use crate::definitions::grammar::Grammar;\n', encoding="utf-8")
            self.assertTrue(guard.source_errors(guard.source_counts(root)))

    def test_backend_files_cannot_hide_without_references(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "crates/intersect-model/src/definitions/grammar"
            source.mkdir(parents=True)
            (source / "hidden.rs").write_text('// backend retained\n', encoding="utf-8")
            self.assertTrue(guard.source_errors(guard.source_counts(root)))

    def test_typed_programs_and_text_name_policy_cannot_hide_in_model(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "crates/intersect-model/src"
            source.mkdir(parents=True)
            for declaration in (
                "pub struct Owner { program: Grammar }",
                "pub type Program = ParserPrediction;",
                "fn language_reference_key() {}",
                "fn language_rule_class_key() {}",
            ):
                with self.subTest(declaration=declaration):
                    (source / "hidden.rs").write_text(declaration, encoding="utf-8")
                    self.assertTrue(guard.source_errors(guard.source_counts(root)))

    def test_retired_model_feature_cannot_be_reintroduced(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text("[workspace]\n", encoding="utf-8")
            self.assertTrue(guard.dependency_errors(root, {"features": {"text-compiler": []}}))

    def test_xtext_cannot_depend_on_library_or_application_harnesses(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(
                '[workspace.dependencies]\nexample = { package = "intersect-library", version = "0.1" }\n',
                encoding="utf-8")
            for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
                for context in ("package", "target"):
                    with self.subTest(kind=kind, context=context):
                        tables = {kind: {"example": {"workspace": True}}}
                        manifest = tables if context == "package" else {"target": {"cfg(unix)": tables}}
                        self.assertTrue(guard.dependency_errors(
                            root, manifest, owner="xtext", forbidden=guard.XTEXT_FORBIDDEN))
            self.assertEqual(guard.dependency_errors(root, {"dependencies": {"intersect-model": "0.1"}},
                                                   owner="xtext", forbidden=guard.XTEXT_FORBIDDEN), [])


if __name__ == "__main__":
    unittest.main()
