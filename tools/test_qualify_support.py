"""Qualification delivery controls; fake child commands never compile Rust."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock
import contextlib
import io
import json
import os
from types import SimpleNamespace

import qualify_support as qualify


class QualificationTests(unittest.TestCase):
    def test_repository_configuration_excludes_foreign_packages(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'repository-checks.json').write_text(json.dumps({
                'scoped_packages': {'model': ['intersect-model'], 'host': ['intersect-run']},
                'feature_controls': [], 'examples': [],
            }), encoding='utf-8')
            plan = qualify.make_plan(root, 'all', None, 'offline', root/'native', root/'wasm', scope='host')
            self.assertNotIn('intersect-web', json.dumps(plan))
            with self.assertRaisesRegex(ValueError, 'does not own'):
                qualify.make_plan(root, 'native', None, 'offline', root/'native', root/'wasm', scope='text')

    def test_scoped_lanes_exclude_workspace_and_milestone_work(self):
        root = Path('/repository')
        for scope, expected in qualify.SCOPED_PACKAGES.items():
            plan = qualify.make_plan(root, 'all', None, 'offline', root/'native', root/'wasm', scope=scope)
            text = json.dumps(plan['commands'])
            self.assertNotIn('--workspace', text)
            self.assertNotIn('milestone-tests', text)
            self.assertNotIn('intersect-semantic-bindings-tests', text)
            quick = next(c for c in plan['commands'] if c['key'] == scope+'-quick-tests')
            self.assertIn('tools/test_report.py', quick['argv'])
            self.assertTrue(all(package in quick['argv'] for package in expected))
            self.assertIn('--lib', quick['argv'])
            self.assertIn('--tests', quick['argv'])
            self.assertTrue(any(c['key'].endswith('wasm-execution') for c in plan['commands']))
            if scope in ('model', 'text'):
                self.assertNotIn('intersect-library', quick['argv'])

    def test_all_lanes_compile_before_costly_checks_and_keep_network_explicit(self):
        root = Path("/repository")
        for lane in ["preflight", "native", "wasm", "all"]:
            plan = qualify.make_plan(root, lane, "stable", "offline",
                                     root / "native-cache", root / "wasm-cache")
            self.assertEqual([c["key"] for c in plan["commands"][:2]],
                             ["format", "strict-all-target-clippy"])
            for command in plan["commands"][1:]:
                if command["argv"][0] == "cargo":
                    self.assertIn("--locked", command["argv"])
                    self.assertIn("--offline", command["argv"])
                    self.assertEqual(command["argv"][1], "+stable")
            if lane in ("wasm", "all"):
                self.assertEqual(plan["commands"][-1]["target_dir"],
                                 str(root / "wasm-cache"))
        online = qualify.make_plan(root, "native", None, "online",
                                   root / "cache", root / "cache")
        self.assertNotIn("--offline", online["commands"][1]["argv"])
        self.assertEqual(len(online["commands"]), 11)

    def execute(self, results, after=None, exception=None):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        output = root / "evidence"
        before = {"head": "original", "files": {"source.rs": {"sha256": "before"}}}
        snapshots = iter([before, after or before])
        calls = []

        def run(argv, **kwargs):
            calls.append((argv, kwargs["env"]["CARGO_NET_OFFLINE"]))
            kwargs["stdout"].write(b"complete child output\n")
            if exception:
                raise exception
            return subprocess.CompletedProcess(argv, results[len(calls) - 1])

        plan = {"network": "offline", "commands": [
            {"key": key, "argv": [sys.executable, key]} for key in ["first", "second"]]}
        code = qualify.run_plan(root, plan, output,
                                snapshot=lambda _: next(snapshots), run=run,
                                metadata=lambda *_: {"fake_toolchain": "test-only"})
        terminal = json.loads((output / "terminal.json").read_text())
        return code, terminal, calls

    def test_command_failure_stops_later_checks_but_still_guards_source(self):
        code, terminal, calls = self.execute([101])
        self.assertEqual(code, 101)
        self.assertEqual(terminal["state"], "fail")
        self.assertEqual(terminal["source_postguard"]["state"], "pass")
        self.assertEqual(terminal["not_run"], ["second"])
        self.assertEqual(len(calls), 1)
        self.assertEqual(calls[0][1], "true")
        self.assertEqual(terminal["commands"][0]["log"]["bytes"], 22)

    def test_source_mutation_rejects_successful_commands(self):
        after = {"head": "original", "files": {"source.rs": {"sha256": "changed"}}}
        code, terminal, calls = self.execute([0, 0], after=after)
        self.assertEqual(code, 1)
        self.assertEqual(len(calls), 2)
        self.assertEqual(terminal["source_postguard"]["state"], "fail")
        self.assertEqual(terminal["source_postguard"]["changed_paths"], ["source.rs"])

    def test_spawn_error_also_writes_terminal_and_postguard(self):
        code, terminal, calls = self.execute([], exception=FileNotFoundError("missing tool"))
        self.assertEqual(code, 1)
        self.assertEqual(terminal["error"]["kind"], "FileNotFoundError")
        self.assertEqual(terminal["source_postguard"]["state"], "pass")
        self.assertEqual(len(calls), 1)

    def test_success_is_limited_to_lane_and_no_gate_award(self):
        code, terminal, calls = self.execute([0, 0])
        self.assertEqual(code, 0)
        self.assertEqual(terminal["state"], "pass")
        self.assertEqual(terminal["gate_closures"], [])
        self.assertEqual(terminal["not_run"], [])
        self.assertEqual(len(calls), 2)


    def test_source_snapshot_discovers_additions_and_deletions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "tracked.rs").write_text("unchanged", encoding="utf-8")
            (root / "untracked.rs").write_text("original", encoding="utf-8")

            def inventory(_, *args):
                if args[0] == "rev-parse":
                    return b"original"
                names = ["tracked.rs"]
                names.extend(path.name for path in root.glob("untracked*.rs"))
                return (chr(0).join(names) + chr(0)).encode("utf-8")

            with mock.patch.object(qualify, "git", side_effect=inventory):
                before = qualify.source_snapshot(root)
                (root / "tracked.rs").unlink()
                (root / "untracked.rs").unlink()
                (root / "untracked-new.rs").write_text("added", encoding="utf-8")
                after = qualify.source_snapshot(root)
            self.assertEqual(qualify.source_difference(before, after)["changed_paths"],
                             ["tracked.rs", "untracked-new.rs", "untracked.rs"])
            self.assertEqual(after["files"]["tracked.rs"]["kind"], "missing")

    def test_plan_never_discovers_tools_sources_or_writes_output(self):
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / "never-created"
            arguments = ["qualify_support.py", "--plan", "--lane", "all",
                         "--output", str(output)]
            with (
                mock.patch.object(sys, "argv", arguments),
                mock.patch.object(qualify.subprocess, "run",
                                  side_effect=AssertionError("plan invoked a child")),
                contextlib.redirect_stdout(io.StringIO()) as captured,
            ):
                self.assertEqual(qualify.main(), 0)
            self.assertEqual(json.loads(captured.getvalue())["lane"], "all")
            self.assertFalse(output.exists())

    def test_output_must_be_ignored_and_cannot_replace_prior_run(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            output = root / "new-run"
            with mock.patch.object(qualify.subprocess, "run") as child:
                child.return_value.returncode = 1
                with self.assertRaisesRegex(ValueError, "ignored"):
                    qualify.validate_output(root, output)
                child.return_value.returncode = 0
                qualify.validate_output(root, output)
            output.mkdir()
            (output / "terminal.json").write_text("original", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "already exists"):
                qualify.validate_output(root, output)
            with self.assertRaises(FileExistsError):
                qualify.run_plan(root, {}, output)
            self.assertEqual((output / "terminal.json").read_text(), "original")

    def test_metadata_error_still_publishes_terminal_and_source_guard(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = {"head": "original", "files": {}}
            with mock.patch.object(qualify, "tool_metadata"):
                def bad_metadata(*_):
                    raise FileNotFoundError("missing cargo")
                code = qualify.run_plan(
                    root, {"network": "offline", "commands": []}, root / "run",
                    snapshot=lambda _: source, metadata=bad_metadata)
            terminal = json.loads((root / "run/terminal.json").read_text())
            self.assertEqual(code, 1)
            self.assertEqual(terminal["source_postguard"]["state"], "pass")
            self.assertEqual(terminal["error"]["kind"], "FileNotFoundError")
            self.assertIn("sha256", terminal["runner"])
            self.assertIn("RUST_MIN_STACK", terminal["build_environment"])



    @unittest.skipUnless(os.name == "nt", "Windows extended-path control")
    def test_real_windows_long_path_is_hashed_not_missing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            name = "/".join(["long-source-" + "x" * 70] * 3 + ["fixture.class"])
            path = qualify.physical_path(root / name)
            self.assertGreater(len(str(root / name)), 300)
            path.parent.mkdir(parents=True)
            path.write_bytes(b"original long-path bytes")
            def inventory(_, *args):
                return b"original" if args[0] == "rev-parse" else name.encode() + bytes([0])
            try:
                with mock.patch.object(qualify, "git", side_effect=inventory):
                    snapshot = qualify.source_snapshot(root)
                self.assertEqual(snapshot["files"][name]["kind"], "file")
                self.assertEqual(snapshot["files"][name]["bytes"], 24)
            finally:
                # Remove only this test's exact leaf and three known empty
                # directories via extended paths; default tempfile cleanup on
                # Windows cannot traverse the deliberately long fixture.
                path.unlink(missing_ok=True)
                for directory_path in [path.parent, path.parent.parent,
                                       path.parent.parent.parent]:
                    directory_path.rmdir()

    def test_source_stat_permission_error_cannot_become_missing(self):
        def inventory(_, *args):
            return b"original" if args[0] == "rev-parse" else b"denied.rs" + bytes([0])
        with mock.patch.object(qualify, "git", side_effect=inventory),                 mock.patch.object(Path, "lstat", side_effect=PermissionError("denied")):
            with self.assertRaises(PermissionError):
                qualify.source_snapshot(Path("/unreadable"))



    def disk_preflight_attempt(self, minimum, wrapped=False):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        root = Path(temporary.name)
        cache = root / "new-cache" / "nested"
        output = root / "run"
        source = {"head": "original", "files": {}}
        plan = {"network": "offline", "minimum_free_space_bytes": minimum,
                "commands": [{"key": "compile", "argv": ["cargo", "test"],
                              "target_dir": str(cache)},
                             {"key": "later", "argv": ["cargo", "run"],
                              "target_dir": str(cache)}]}
        if wrapped:
            plan['commands'][0]['argv'] = [sys.executable, '-B', 'tools/test_report.py', 'run']
            plan['commands'][0]['uses_cargo'] = True
        def run(argv, **kwargs):
            kwargs["stdout"].write(b"fake command output")
            return subprocess.CompletedProcess(argv, 0)
        child = mock.Mock(side_effect=run)
        with mock.patch.object(qualify.shutil, "disk_usage",
                               return_value=SimpleNamespace(free=100, used=900, total=1000)):
            code = qualify.run_plan(root, plan, output, snapshot=lambda _: source,
                                    metadata=lambda *_: {}, run=child)
        return code, json.loads((output / "terminal.json").read_text()), child, root, cache

    def test_low_disk_prevents_cargo_and_retains_terminal_postguard(self):
        code, report, child, root, cache = self.disk_preflight_attempt(101)
        self.assertEqual(code, 1)
        child.assert_not_called()
        self.assertEqual(report["state"], "fail")
        self.assertEqual(report["error"]["kind"], "InsufficientCacheFreeSpace")
        self.assertEqual(report["not_run"], ["compile", "later"])
        self.assertEqual(report["source_postguard"]["state"], "pass")
        observation = report["cache_observations"][0]
        self.assertEqual(observation["free_bytes"], 100)
        self.assertEqual(observation["minimum_free_bytes"], 101)
        self.assertFalse(observation["threshold_pass"])
        self.assertEqual(observation["probe_path"], str(root.resolve()))
        self.assertFalse(cache.exists())

    def test_reporter_wrapped_cargo_keeps_the_cache_space_guard(self):
        code, report, child, _, _ = self.disk_preflight_attempt(101, wrapped=True)
        self.assertEqual(code, 1)
        child.assert_not_called()
        self.assertEqual(report['error']['kind'], 'InsufficientCacheFreeSpace')

    def test_default_disk_threshold_only_records_space_and_allows_commands(self):
        code, report, child, root, cache = self.disk_preflight_attempt(None)
        self.assertEqual(code, 0)
        self.assertEqual(child.call_count, 2)
        self.assertEqual(len(report["cache_observations"]), 2)
        for command in report["commands"]:
            self.assertEqual(command["cache_volume"]["free_bytes"], 100)
            self.assertIsNone(command["cache_volume"]["minimum_free_bytes"])
            self.assertTrue(command["cache_volume"]["threshold_pass"])
        self.assertEqual(report["source_postguard"]["state"], "pass")
        self.assertIn("CARGO_PROFILE_TEST_STRIP", report["build_environment"])
        self.assertFalse(cache.exists())


if __name__ == "__main__":
    unittest.main()
