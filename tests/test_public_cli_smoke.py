import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
default_binary_name = "kept.exe" if os.name == "nt" else "kept"
BINARY = Path(os.environ.get("KEPT_BIN", ROOT / "target" / "debug" / default_binary_name))


class PublicCliSmokeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not BINARY.is_file():
            raise RuntimeError(
                f"kept binary not found at {BINARY}; run cargo build -p kept-cli first"
            )

    def run_kept(self, *arguments: str, environment: dict[str, str]) -> subprocess.CompletedProcess[str]:
        environment = environment.copy()
        environment["NO_COLOR"] = "1"
        return subprocess.run(
            [str(BINARY), *arguments],
            check=False,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def environment_for(self, directory: Path) -> dict[str, str]:
        environment = os.environ.copy()
        config_home = str(directory / "config-home")
        environment["XDG_CONFIG_HOME"] = config_home
        environment["APPDATA"] = config_home
        # NOTE-001: ใช้ executable ที่ไม่ interactive เพื่อพิสูจน์ command `config edit` โดยไม่เปิด editor จริงใน smoke test
        if os.name == "nt":
            helper = directory / "mock_editor.cmd"
            helper.write_text("@exit 0\n", encoding="utf-8")
            environment["KEPT_EDITOR"] = str(helper)
        else:
            environment["KEPT_EDITOR"] = "/bin/true"
        return environment

    def test_help_surface_has_only_user_facing_public_commands(self) -> None:
        environment = os.environ.copy()
        result = self.run_kept("--help", environment=environment)

        self.assertEqual(result.returncode, 0, result.stderr)
        for command in (
            "scan",
            "find",
            "search",
            "group",
            "convert",
            "setup",
            "config",
            "doctor",
            "review",
            "duplicates",
        ):
            self.assertIn(command, result.stdout)
        self.assertNotIn("NOTE-001", result.stdout)
        self.assertNotIn("\n  policy", result.stdout)
        for command in (
            ("scan", "--help"),
            ("find", "--help"),
            ("search", "--help"),
            ("convert", "--help"),
            ("setup", "--help"),
            ("config", "--help"),
            ("config", "fields", "--help"),
            ("config", "defaults", "--help"),
            ("config", "defaults", "set", "--help"),
            ("config", "profile", "--help"),
            ("config", "profile", "set", "--help"),
            ("config", "scope", "--help"),
            ("config", "scope", "setting", "--help"),
            ("config", "edit", "--help"),
            ("group", "--help"),
            ("group", "types", "--help"),
            ("group", "field", "--help"),
            ("group", "field", "add", "--help"),
            ("doctor", "--help"),
            ("review", "--help"),
            ("duplicates", "--help"),
        ):
            command_help = self.run_kept(*command, environment=environment)
            self.assertEqual(command_help.returncode, 0, command_help.stderr)
            self.assertIn("Usage: kept", command_help.stdout)

    def test_setup_config_doctor_and_fix_use_one_user_owned_config(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            environment = self.environment_for(directory)
            config_path = directory / "config-home" / "kept" / "config.yaml"

            setup = self.run_kept("setup", environment=environment)
            self.assertEqual(setup.returncode, 0, setup.stderr)
            self.assertTrue(config_path.is_file())
            original = config_path.read_text(encoding="utf-8")

            second_setup = self.run_kept("setup", environment=environment)
            self.assertEqual(second_setup.returncode, 0, second_setup.stderr)
            self.assertEqual(config_path.read_text(encoding="utf-8"), original)

            config = self.run_kept("config", environment=environment)
            self.assertEqual(config.returncode, 0, config.stderr)
            self.assertIn(str(config_path), config.stdout)
            self.assertIn("Profiles:", config.stdout)
            self.assertIn("Scopes:", config.stdout)
            self.assertIn("kept config profile list", config.stdout)

            fields = self.run_kept("config", "fields", environment=environment)
            self.assertEqual(fields.returncode, 0, fields.stderr)
            self.assertIn("Default value", fields.stdout)
            self.assertIn("case", fields.stdout)
            self.assertIn("preserve", fields.stdout)
            self.assertIn("shortcuts.<token>", fields.stdout)

            edit = self.run_kept("config", "edit", environment=environment)
            self.assertEqual(edit.returncode, 0, edit.stderr)

            doctor = self.run_kept("doctor", environment=environment)
            self.assertEqual(doctor.returncode, 0, doctor.stderr)
            self.assertIn("OK CONFIG_VALID", doctor.stdout)
            self.assertIn("OK CONFIG_EDITOR_AVAILABLE", doctor.stdout)

            invalid_config = "version: [broken\n"
            config_path.write_text(invalid_config, encoding="utf-8")
            invalid_doctor = self.run_kept("doctor", environment=environment)
            self.assertNotEqual(invalid_doctor.returncode, 0)
            self.assertIn("CONFIG_INVALID", invalid_doctor.stdout)

            repair = self.run_kept("doctor", "--fix", environment=environment)
            self.assertEqual(repair.returncode, 0, repair.stderr)
            backup_path = config_path.with_name("config.yaml.invalid.bak")
            self.assertEqual(backup_path.read_text(encoding="utf-8"), invalid_config)
            self.assertIn("Repaired config", repair.stdout)

    def test_config_profile_lifecycle_writes_only_user_owned_config(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            environment = self.environment_for(directory)
            config_path = directory / "config-home" / "kept" / "config.yaml"

            self.assertEqual(self.run_kept("setup", environment=environment).returncode, 0)
            initial_list = self.run_kept("config", "profile", "list", environment=environment)
            self.assertEqual(initial_list.returncode, 0, initial_list.stderr)
            self.assertIn("generic", initial_list.stdout)

            add = self.run_kept("config", "profile", "add", "invoices", environment=environment)
            self.assertEqual(add.returncode, 0, add.stderr)
            self.assertIn("invoices", config_path.read_text(encoding="utf-8"))

            set_case = self.run_kept(
                "config", "profile", "set", "invoices", "case", "lower", environment=environment
            )
            self.assertEqual(set_case.returncode, 0, set_case.stderr)
            shown = self.run_kept("config", "profile", "show", "invoices", environment=environment)
            self.assertEqual(shown.returncode, 0, shown.stderr)
            self.assertIn("case: lower", shown.stdout)

            unset_case = self.run_kept(
                "config", "profile", "unset", "invoices", "case", environment=environment
            )
            self.assertEqual(unset_case.returncode, 0, unset_case.stderr)
            self.assertNotIn("case: lower", config_path.read_text(encoding="utf-8"))

            remove = self.run_kept("config", "profile", "remove", "invoices", environment=environment)
            self.assertEqual(remove.returncode, 0, remove.stderr)
            self.assertNotIn("invoices", config_path.read_text(encoding="utf-8"))

    def test_config_profile_set_supports_catalog_fields_without_yaml_editing(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            environment = self.environment_for(directory)
            self.assertEqual(self.run_kept("setup", environment=environment).returncode, 0)
            self.assertEqual(
                self.run_kept("config", "profile", "add", "catalog-test", environment=environment).returncode,
                0,
            )
            for field, value in (
                ("separator", "kebab"),
                ("extensions", "pdf,docx"),
                ("numbers.maxDigitsPerToken", "6"),
                ("shortcuts.rpt", "report"),
                ("aliases.qtr", "quarter"),
                ("variables.project", "kept"),
                ("prefix.required", "{{project}}"),
                ("similarity.name.threshold", "0.85"),
                ("similarity.name.caseSensitive", "true"),
            ):
                result = self.run_kept(
                    "config", "profile", "set", "catalog-test", field, value, environment=environment
                )
                self.assertEqual(result.returncode, 0, f"{field}: {result.stderr}")

            shown = self.run_kept("config", "profile", "show", "catalog-test", environment=environment)
            self.assertEqual(shown.returncode, 0, shown.stderr)
            for expected in ("separator: kebab", "pdf", "maxDigitsPerToken: 6", "rpt: report", "qtr: quarter", "project: kept", "threshold: 0.85", "caseSensitive: true"):
                self.assertIn(expected, shown.stdout)

            unset = self.run_kept(
                "config", "profile", "unset", "catalog-test", "shortcuts.rpt", environment=environment
            )
            self.assertEqual(unset.returncode, 0, unset.stderr)
            self.assertNotIn("rpt: report", self.run_kept("config", "profile", "show", "catalog-test", environment=environment).stdout)

    def test_config_scope_lifecycle_uses_absolute_user_selected_paths(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            environment = self.environment_for(directory)
            root = directory / "workspace"
            root.mkdir()

            self.assertEqual(self.run_kept("setup", environment=environment).returncode, 0)
            self.assertEqual(
                self.run_kept("config", "profile", "add", "invoices", environment=environment).returncode,
                0,
            )
            add = self.run_kept(
                "config", "scope", "add", "invoices", str(root), "--priority", "50", environment=environment
            )
            self.assertEqual(add.returncode, 0, add.stderr)

            listed = self.run_kept("config", "scope", "list", environment=environment)
            self.assertEqual(listed.returncode, 0, listed.stderr)
            self.assertIn(str(root), listed.stdout)
            self.assertIn("50", listed.stdout)

            set_priority = self.run_kept(
                "config", "scope", "set", str(root), "--priority", "200", environment=environment
            )
            self.assertEqual(set_priority.returncode, 0, set_priority.stderr)
            shown = self.run_kept("config", "scope", "show", str(root), environment=environment)
            self.assertEqual(shown.returncode, 0, shown.stderr)
            self.assertIn("priority: 200", shown.stdout)

            exception = root / "generated"
            add_exception = self.run_kept(
                "config", "scope", "exception", "add", str(root), str(exception), environment=environment
            )
            self.assertEqual(add_exception.returncode, 0, add_exception.stderr)
            self.assertIn(str(exception), self.run_kept("config", "scope", "show", str(root), environment=environment).stdout)

            remove_exception = self.run_kept(
                "config", "scope", "exception", "remove", str(root), str(exception), environment=environment
            )
            self.assertEqual(remove_exception.returncode, 0, remove_exception.stderr)
            remove = self.run_kept("config", "scope", "remove", str(root), environment=environment)
            self.assertEqual(remove.returncode, 0, remove.stderr)
            self.assertNotIn(str(root), self.run_kept("config", "scope", "list", environment=environment).stdout)

    def test_scan_find_review_and_duplicates_share_a_persisted_index(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            environment = self.environment_for(directory)
            root = directory / "workspace"
            root.mkdir()
            (root / "Q1_Report.pdf").write_text("report", encoding="utf-8")
            (root / "duplicate-left.txt").write_text("same", encoding="utf-8")
            (root / "duplicate-right.txt").write_text("same", encoding="utf-8")
            (root / "archive").mkdir()
            (root / "archive" / "notes.txt").write_text("notes", encoding="utf-8")
            (root / ".hidden.txt").write_text("hidden", encoding="utf-8")
            snapshot = directory / "workspace-index.json"
            config_path = directory / "config-home" / "kept" / "config.yaml"
            config_path.parent.mkdir(parents=True)
            config_path.write_text(
                "\n".join(
                    [
                        "version: 1",
                        "defaults:",
                        "  naming: {}",
                        "profiles:",
                        "  reports:",
                        "    naming:",
                        "      case: lower",
                        "      separator: kebab",
                        "      extensions: [pdf]",
                        "scopes:",
                        f"  - path: {json.dumps(str(root.resolve()))}",
                        "    profile: reports",
                        "    recursive: true",
                        "    priority: 100",
                        "",
                    ]
                ),
                encoding="utf-8",
            )

            default_scan = self.run_kept("scan", str(root), "--json", environment=environment)
            self.assertEqual(default_scan.returncode, 0, default_scan.stderr)
            self.assertEqual(json.loads(default_scan.stdout)["fileCount"], 4)

            scan = self.run_kept(
                "scan",
                str(root),
                "--include-hidden",
                "--output",
                str(snapshot),
                "--json",
                environment=environment,
            )
            self.assertEqual(scan.returncode, 0, scan.stderr)
            self.assertEqual(json.loads(scan.stdout)["fileCount"], 5)
            self.assertTrue(snapshot.is_file())

            find = self.run_kept(
                "find",
                str(root),
                "--type",
                "pdf",
                "--index",
                str(snapshot),
                "--json",
                environment=environment,
            )
            self.assertEqual(find.returncode, 0, find.stderr)
            self.assertEqual([record["path"] for record in json.loads(find.stdout)], ["Q1_Report.pdf"])

            default_review = self.run_kept("review", str(root), environment=environment)
            self.assertEqual(default_review.returncode, 0, default_review.stderr)

            named_find = self.run_kept(
                "find", str(root), "--name", "Report", "--json", environment=environment
            )
            self.assertEqual(named_find.returncode, 0, named_find.stderr)
            self.assertEqual([record["path"] for record in json.loads(named_find.stdout)], ["Q1_Report.pdf"])

            review = self.run_kept("review", str(root), "--index", str(snapshot), environment=environment)
            self.assertEqual(review.returncode, 0, review.stderr)
            self.assertIn("Naming policy: 1 finding(s)", review.stdout)
            self.assertIn("proposed: q1-report.pdf", review.stdout)
            self.assertNotIn("rename applied", review.stdout)

            filtered_find = self.run_kept(
                "find",
                str(root),
                "--path",
                "archive",
                "--min-size",
                "1b",
                "--max-size",
                "100b",
                "--after",
                "0",
                "--before",
                "4102444800",
                "--json",
                environment=environment,
            )
            self.assertEqual(filtered_find.returncode, 0, filtered_find.stderr)
            self.assertEqual([record["path"].replace("\\", "/") for record in json.loads(filtered_find.stdout)], ["archive/notes.txt"])

            default_duplicates = self.run_kept("duplicates", str(root), environment=environment)
            self.assertEqual(default_duplicates.returncode, 0, default_duplicates.stderr)
            self.assertIn("same_content", default_duplicates.stdout)

            default_export = self.run_kept(
                "duplicates", str(root), "--action", "export-plan", "--yes", environment=environment
            )
            self.assertEqual(default_export.returncode, 0, default_export.stderr)
            self.assertTrue((root / "kept-duplicate-plan.json").is_file())

            duplicates = self.run_kept(
                "duplicates",
                str(root),
                "--index",
                str(snapshot),
                "--json",
                environment=environment,
            )
            self.assertEqual(duplicates.returncode, 0, duplicates.stderr)
            groups = json.loads(duplicates.stdout)["groups"]
            self.assertTrue(any(set(group["items"]) == {"duplicate-left.txt", "duplicate-right.txt"} for group in groups))

            export = self.run_kept(
                "duplicates",
                str(root),
                "--index",
                str(snapshot),
                "--action",
                "export-plan",
                "--yes",
                environment=environment,
            )
            self.assertEqual(export.returncode, 0, export.stderr)
            self.assertTrue((root / "kept-duplicate-plan.json").is_file())

    def test_config_defaults_show_set_and_unset_without_yaml_editing(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            environment = self.environment_for(directory)
            self.assertEqual(self.run_kept("setup", environment=environment).returncode, 0)
            initial = self.run_kept("config", "defaults", "show", environment=environment)
            self.assertEqual(initial.returncode, 0, initial.stderr)
            self.assertIn("case: preserve", initial.stdout)

            set_default = self.run_kept(
                "config", "defaults", "set", "separator", "kebab", environment=environment
            )
            self.assertEqual(set_default.returncode, 0, set_default.stderr)
            shown = self.run_kept("config", "defaults", "show", environment=environment)
            self.assertIn("separator: kebab", shown.stdout)

            unset_default = self.run_kept(
                "config", "defaults", "unset", "separator", environment=environment
            )
            self.assertEqual(unset_default.returncode, 0, unset_default.stderr)
            self.assertNotIn("separator: kebab", self.run_kept("config", "defaults", "show", environment=environment).stdout)

    def test_config_scope_setting_overrides_one_folder_without_new_profile(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            environment = self.environment_for(directory)
            root = directory / "workspace"
            root.mkdir()
            self.assertEqual(self.run_kept("setup", environment=environment).returncode, 0)
            self.assertEqual(
                self.run_kept("config", "scope", "add", "reports", str(root), environment=environment).returncode,
                0,
            )
            set_override = self.run_kept(
                "config", "scope", "setting", "set", str(root), "separator", "snake", environment=environment
            )
            self.assertEqual(set_override.returncode, 0, set_override.stderr)
            shown = self.run_kept("config", "scope", "show", str(root), environment=environment)
            self.assertEqual(shown.returncode, 0, shown.stderr)
            self.assertIn("separator: snake", shown.stdout)

            unset_override = self.run_kept(
                "config", "scope", "setting", "unset", str(root), "separator", environment=environment
            )
            self.assertEqual(unset_override.returncode, 0, unset_override.stderr)
            self.assertNotIn("separator: snake", self.run_kept("config", "scope", "show", str(root), environment=environment).stdout)

    def test_registry_group_lifecycle_explains_types_and_preserves_order(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            environment = self.environment_for(directory)
            csv_path = directory / "keywords.csv"
            registry_path = directory / "registry.json"
            csv_path.write_text("id,aliases\nreport,report|รายงาน\n", encoding="utf-8")
            imported = self.run_kept(
                "registry", "import", "--csv", str(csv_path), "--group-id", "product_terms",
                "--group-name", "Product terms", "--output", str(registry_path), environment=environment,
            )
            self.assertEqual(imported.returncode, 0, imported.stderr)

            types = self.run_kept("group", "types", environment=environment)
            self.assertEqual(types.returncode, 0, types.stderr)
            self.assertIn("string", types.stdout)
            self.assertIn("array", types.stdout)
            self.assertIn("boolean", types.stdout)

            added = self.run_kept(
                "group", "add", str(registry_path), "finance_terms", "--name", "Finance terms",
                "--description", "ศัพท์การเงิน", environment=environment,
            )
            self.assertEqual(added.returncode, 0, added.stderr)
            updated = self.run_kept(
                "group", "set", str(registry_path), "finance_terms", "--name", "Finance vocabulary",
                "--description", "คำศัพท์บัญชี", environment=environment,
            )
            self.assertEqual(updated.returncode, 0, updated.stderr)
            listed = self.run_kept("group", "list", str(registry_path), environment=environment)
            self.assertEqual(listed.returncode, 0, listed.stderr)
            self.assertIn("product_terms", listed.stdout)
            self.assertIn("finance_terms", listed.stdout)
            self.assertIn("Finance vocabulary", listed.stdout)

            moved = self.run_kept(
                "group", "move", str(registry_path), "finance_terms", "--before", "product_terms", environment=environment
            )
            self.assertEqual(moved.returncode, 0, moved.stderr)
            listed_after_move = self.run_kept("group", "list", str(registry_path), environment=environment)
            self.assertLess(listed_after_move.stdout.index("finance_terms"), listed_after_move.stdout.index("product_terms"))

            removed = self.run_kept("group", "remove", str(registry_path), "finance_terms", environment=environment)
            self.assertEqual(removed.returncode, 0, removed.stderr)
            self.assertNotIn("finance_terms", self.run_kept("group", "list", str(registry_path), environment=environment).stdout)

    def test_registry_group_field_schema_lifecycle_uses_supported_types(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            environment = self.environment_for(directory)
            csv_path = directory / "keywords.csv"
            registry_path = directory / "registry.json"
            csv_path.write_text("id,aliases\nreport,report|รายงาน\n", encoding="utf-8")
            self.assertEqual(
                self.run_kept(
                    "registry", "import", "--csv", str(csv_path), "--group-id", "product_terms",
                    "--group-name", "Product terms", "--output", str(registry_path), environment=environment,
                ).returncode,
                0,
            )
            self.assertEqual(
                self.run_kept(
                    "group", "add", str(registry_path), "finance", "--name", "Finance", environment=environment
                ).returncode,
                0,
            )
            added = self.run_kept(
                "group", "field", "add", str(registry_path), "finance", "currency", "enum",
                "--values", "THB,USD", "--required", environment=environment,
            )
            self.assertEqual(added.returncode, 0, added.stderr)
            fields = self.run_kept("group", "field", "list", str(registry_path), "finance", environment=environment)
            self.assertEqual(fields.returncode, 0, fields.stderr)
            self.assertIn("currency", fields.stdout)
            self.assertIn("enum", fields.stdout)
            self.assertIn("THB,USD", fields.stdout)

            removed = self.run_kept(
                "group", "field", "remove", str(registry_path), "finance", "currency", environment=environment
            )
            self.assertEqual(removed.returncode, 0, removed.stderr)
            self.assertNotIn("currency", self.run_kept("group", "field", "list", str(registry_path), "finance", environment=environment).stdout)

    def test_search_and_convert_public_commands(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            environment = self.environment_for(directory)
            csv_path = directory / "keywords.csv"
            registry_path = directory / "registry.json"
            markdown_path = directory / "source.md"
            ir_path = directory / "result.json"
            csv_path.write_text("id,aliases\nreport,report|รายงาน\n", encoding="utf-8")
            markdown_path.write_text("# Report\n\nBody\n", encoding="utf-8")

            imported = self.run_kept(
                "registry",
                "import",
                "--csv",
                str(csv_path),
                "--group-id",
                "product_terms",
                "--group-name",
                "Product terms",
                "--output",
                str(registry_path),
                environment=environment,
            )
            self.assertEqual(imported.returncode, 0, imported.stderr)
            self.assertTrue(registry_path.is_file())

            search = self.run_kept(
                "search",
                str(registry_path),
                "รายงาน",
                "--group",
                "product_terms",
                "--json",
                environment=environment,
            )
            self.assertEqual(search.returncode, 0, search.stderr)
            self.assertTrue(any(result["id"] == "report" for result in json.loads(search.stdout)))

            convert = self.run_kept("convert", str(markdown_path), str(ir_path), environment=environment)
            self.assertEqual(convert.returncode, 0, convert.stderr)
            self.assertTrue(ir_path.is_file())
            document = json.loads(ir_path.read_text(encoding="utf-8"))
            self.assertIn("metadata", document)
            self.assertEqual(document["blocks"][0]["type"], "heading")


if __name__ == "__main__":
    unittest.main()
