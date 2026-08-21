import json
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


class RepositoryContractTests(unittest.TestCase):
    def test_root_entrypoints_and_source_of_truth_are_present(self) -> None:
        for relative_path in (
            "SPEC.md",
            "AGENTS.md",
            "Justfile",
            "schema/keyword-registry.schema.json",
            "schema/README.md",
            "research/PONYTAIL_SOURCE_FINDINGS.md",
            "research/PONYTAIL_TESTING_BENCHMARK_YAGNI_APPLICABILITY.md",
            "research/PDF_INSPECTOR_RESEARCH.md",
            "benchmarks/README.md",
            "benchmarks/data/search_duplicate_release.jsonl",
            "benchmarks/charts/search_duplicate_release.png",
            ".agents/skills/repository-operating-recovery/SKILL.md",
            ".agents/skills/repository-operating-recovery/scripts/repository_inventory.py",
        ):
            self.assertTrue(
                (ROOT / relative_path).is_file(),
                f"missing repository source-of-truth artifact: {relative_path}",
            )

    def test_public_registry_schema_is_current_and_machine_readable(self) -> None:
        schema_path = ROOT / "schema/keyword-registry.schema.json"
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
        self.assertEqual(schema["$schema"], "http://json-schema.org/draft-07/schema#")
        self.assertIn("version", schema["properties"])
        self.assertIn("FoundationProfile", schema["definitions"])

    def test_active_tree_does_not_mix_reports_and_snapshots_into_docs(self) -> None:
        docs = ROOT / "docs"
        self.assertTrue((docs / "adr").is_dir(), "docs/adr is the only active docs subtree")
        unexpected_files = [
            path.relative_to(docs)
            for path in docs.rglob("*")
            if path.is_file() and "adr" not in path.relative_to(docs).parts
        ]
        self.assertEqual(unexpected_files, [])
        self.assertFalse((ROOT / "scripts" / "create_evidence_gif.py").exists())

    def test_agent_requirement_ledger_is_retained_and_read_before_work(self) -> None:
        ledger_path = ROOT / ".agents" / "MEMORY.md"
        agents = (ROOT / "AGENTS.md").read_text(encoding="utf-8")

        self.assertTrue(
            ledger_path.is_file(),
            "agent-only requirement ledger must survive between sessions",
        )
        self.assertIn("Requirement ID", ledger_path.read_text(encoding="utf-8"))
        self.assertIn(".agents/MEMORY.md", agents)
        self.assertIn("evidence", agents.lower())

    def test_agent_learning_remains_a_project_asset(self) -> None:
        learning_files = list((ROOT / ".learnings").rglob("*"))
        self.assertTrue(
            any(path.is_file() for path in learning_files),
            ".learnings must remain in the source tree for future agent sessions",
        )

    def test_research_decisions_have_adr_and_open_choices_have_one_plan(self) -> None:
        expected = (
            "docs/adr/0001_deterministic_evidence_benchmark_contract.md",
            "docs/adr/0002_optional_pdf_inspection_adapter.md",
            "plan.md",
        )
        for relative_path in expected:
            self.assertTrue(
                (ROOT / relative_path).is_file(),
                f"missing decision or planning artifact: {relative_path}",
            )

    def test_release_workflow_has_monotonic_tag_guard_and_dual_archives(self) -> None:
        workflow = (ROOT / ".github" / "workflows" / "release.yml").read_text(encoding="utf-8")

        self.assertIn("v*.*.*", workflow)
        self.assertIn("tools/release_contract.py", workflow)
        self.assertIn("just check", workflow)
        self.assertIn("source.tar.gz", workflow)
        self.assertIn("source.zip", workflow)
        self.assertIn("gh release create", workflow)

    def test_entrypoints_have_non_overlapping_responsibilities(self) -> None:
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        agents = (ROOT / "AGENTS.md").read_text(encoding="utf-8")
        todo = (ROOT / "TODO.md").read_text(encoding="utf-8")

        self.assertNotIn("## Supported now", readme)
        self.assertNotIn("## Not supported", readme)
        self.assertNotIn("## Commands", agents)
        self.assertIn("## Why bl1nk-kept", readme)
        self.assertIn("## P0 — Repository operating baseline", todo)
        self.assertNotIn("completion report", todo.lower())

    def test_kept_doc_is_a_pure_library_and_the_mcp_server_has_its_own_crate(self) -> None:
        doc_manifest = (ROOT / "crate" / "kept-doc" / "Cargo.toml").read_text(
            encoding="utf-8"
        )
        self.assertNotIn(
            "[[bin]]",
            doc_manifest,
            "kept-doc must not ship binaries; the user CLI lives in kept-cli and the MCP server in kept-mcp",
        )
        mcp_manifest_path = ROOT / "crate" / "kept-mcp" / "Cargo.toml"
        self.assertTrue(mcp_manifest_path.is_file(), "kept-mcp crate must exist")
        mcp_manifest = mcp_manifest_path.read_text(encoding="utf-8")
        self.assertEqual(mcp_manifest.count("[[bin]]"), 1)
        self.assertIn('name = "bl1nk-kept-mcp"', mcp_manifest)

        workspace = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
        self.assertIn('"crate/kept-mcp"', workspace)


if __name__ == "__main__":
    unittest.main()
