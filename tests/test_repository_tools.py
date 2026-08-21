import tempfile
import unittest
from pathlib import Path

from tools.check_markdown_links import find_broken_local_links
from tools.check_version_contract import find_version_contract_errors


class RepositoryToolTests(unittest.TestCase):
    def test_link_checker_reports_only_missing_local_markdown_targets(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "SPEC.md").write_text("# spec\n", encoding="utf-8")
            source = root / "README.md"
            source.write_text(
                "[spec](SPEC.md) [missing](missing.md) [external](https://example.com)\n",
                encoding="utf-8",
            )

            broken = find_broken_local_links(root)

            self.assertEqual(broken, [(source, "missing.md")])

    def test_version_checker_requires_workspace_changelog_and_spec_to_agree(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(
                '[workspace.package]\nversion = "0.2.0"\n', encoding="utf-8"
            )
            (root / "CHANGELOG.md").write_text("## [0.2.0]\n", encoding="utf-8")
            (root / "SPEC.md").write_text(
                "**Workspace package version:** `0.2.0`\n", encoding="utf-8"
            )

            self.assertEqual(find_version_contract_errors(root), [])

            (root / "SPEC.md").write_text(
                "**Workspace package version:** `0.1.0`\n", encoding="utf-8"
            )
            self.assertEqual(
                find_version_contract_errors(root),
                ["SPEC.md does not declare workspace version 0.2.0"],
            )


    def test_release_contract_requires_matching_and_strictly_newer_semver_tag(self) -> None:
        from tools.release_contract import ReleaseContractError, validate_release_tag

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(
                '[workspace.package]\nversion = "0.3.0"\n', encoding="utf-8"
            )

            self.assertEqual(validate_release_tag(root, "v0.3.0", ["v0.1.0", "v0.2.0"]), "0.3.0")
            with self.assertRaises(ReleaseContractError):
                validate_release_tag(root, "v0.2.0", ["v0.1.0", "v0.2.0"])
            with self.assertRaises(ReleaseContractError):
                validate_release_tag(root, "v0.2.9", ["v0.1.0", "v0.2.0"])
            with self.assertRaises(ReleaseContractError):
                validate_release_tag(root, "release-0.3.0", ["v0.1.0"])

    def test_version_updater_changes_workspace_spec_and_unreleased_heading(self) -> None:
        from tools.bump_version import update_version

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(
                '[workspace.package]\nversion = "0.2.0"\n', encoding="utf-8"
            )
            (root / "SPEC.md").write_text(
                "**Workspace package version:** `0.2.0`\n", encoding="utf-8"
            )
            (root / "CHANGELOG.md").write_text(
                "# Changelog\n\n## [Unreleased]\n\n## [0.2.0]\n",
                encoding="utf-8",
            )

            update_version(root, "0.3.0")

            self.assertIn('version = "0.3.0"', (root / "Cargo.toml").read_text(encoding="utf-8"))
            self.assertIn(
                "**Workspace package version:** `0.3.0`",
                (root / "SPEC.md").read_text(encoding="utf-8"),
            )
            self.assertIn("## [0.3.0]", (root / "CHANGELOG.md").read_text(encoding="utf-8"))

    def test_version_updater_without_argument_increments_the_patch_release(self) -> None:
        from tools.bump_version import update_version

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text(
                '[workspace.package]\nversion = "0.2.0"\n', encoding="utf-8"
            )
            (root / "SPEC.md").write_text(
                "**Workspace package version:** `0.2.0`\n", encoding="utf-8"
            )
            (root / "CHANGELOG.md").write_text(
                "# Changelog\n\n## [Unreleased]\n\n## [0.2.0]\n",
                encoding="utf-8",
            )

            update_version(root)

            self.assertIn('version = "0.2.1"', (root / "Cargo.toml").read_text(encoding="utf-8"))
            self.assertIn(
                "**Workspace package version:** `0.2.1`",
                (root / "SPEC.md").read_text(encoding="utf-8"),
            )
            self.assertIn("## [0.2.1]", (root / "CHANGELOG.md").read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()


class SourcePackageToolTests(unittest.TestCase):
    def test_source_package_keeps_learning_and_research_but_excludes_rebuildable_artifacts(self) -> None:
        from tools.package_source import create_source_archive
        import zipfile

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "project"
            root.mkdir()
            (root / ".learnings").mkdir()
            (root / ".learnings" / "ERRORS.md").write_text("memory\n", encoding="utf-8")
            (root / "research").mkdir()
            (root / "research" / "source.md").write_text("research\n", encoding="utf-8")
            (root / "target").mkdir()
            (root / "target" / "artifact").write_text("build\n", encoding="utf-8")
            (root / "presentation").mkdir()
            (root / "presentation" / "old.html").write_text("slide\n", encoding="utf-8")
            (root / "dist").mkdir()
            (root / "dist" / "previous.zip").write_bytes(b"previous")
            (root / "__pycache__").mkdir()
            (root / "__pycache__" / "cache.pyc").write_bytes(b"cache")
            (root / "SPEC.md").write_text("spec\n", encoding="utf-8")

            archive = Path(directory) / "source.zip"
            create_source_archive(root, archive)

            with zipfile.ZipFile(archive) as package:
                names = set(package.namelist())
            self.assertIn("project/.learnings/ERRORS.md", names)
            self.assertIn("project/research/source.md", names)
            self.assertIn("project/SPEC.md", names)
            self.assertFalse(any("/target/" in name for name in names))
            self.assertFalse(any("/presentation/" in name for name in names))
            self.assertFalse(any("/dist/" in name for name in names))
            self.assertFalse(any("/__pycache__/" in name for name in names))

    def test_source_tarball_keeps_the_same_handoff_boundary(self) -> None:
        from tools.package_source import create_source_tarball
        import tarfile

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "project"
            root.mkdir()
            (root / ".learnings").mkdir()
            (root / ".learnings" / "ERRORS.md").write_text("memory\n", encoding="utf-8")
            (root / "research").mkdir()
            (root / "research" / "source.md").write_text("research\n", encoding="utf-8")
            (root / "target").mkdir()
            (root / "target" / "artifact").write_text("build\n", encoding="utf-8")

            archive = Path(directory) / "source.tar.gz"
            create_source_tarball(root, archive)

            with tarfile.open(archive, "r:gz") as package:
                names = set(package.getnames())
            self.assertIn("project/.learnings/ERRORS.md", names)
            self.assertIn("project/research/source.md", names)
            self.assertFalse(any("/target/" in name for name in names))
