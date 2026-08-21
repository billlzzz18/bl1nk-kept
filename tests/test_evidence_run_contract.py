import os
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def fixture_dir(name: str) -> Path:
    directory = ROOT / "target" / f"tdd-{name}-{os.getpid()}"
    directory.mkdir(parents=True, exist_ok=True)
    return directory


def run_kept(*arguments: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["cargo", "run", "--quiet", "-p", "kept-cli", "--", *arguments],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )


class EvidenceRunContractTests(unittest.TestCase):
    def test_run_materializes_immutable_manifest_and_raw_jsonl(self) -> None:
        root = fixture_dir("evidence-run")
        (root / "source.md").write_text("รายงานฉบับสมบูรณ์\n", encoding="utf-8")
        result = run_kept(
            "evidence", "run", "--manifest", str(root / "run-manifest.json"),
            "--raw-jsonl", str(root / "raw.jsonl"), "--input", str(root / "source.md")
        )

        self.assertEqual(result.returncode, 0, result.stderr)

    def test_run_with_missing_input_is_rejected_without_creating_a_manifest(self) -> None:
        root = fixture_dir("evidence-run-missing")
        result = run_kept(
            "evidence", "run", "--manifest", str(root / "run-manifest.json"),
            "--raw-jsonl", str(root / "raw.jsonl"), "--input", str(root / "missing.md")
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("refusing to create an immutable manifest", result.stderr)
        self.assertFalse((root / "run-manifest.json").exists())

    def test_offline_rescore_reads_preserved_raw_jsonl_without_rescanning(self) -> None:
        root = fixture_dir("evidence-rescore")
        (root / "run-manifest.json").write_text(
            '{"contract":"evidence-v1","immutable":true}\n', encoding="utf-8"
        )
        (root / "raw.jsonl").write_text(
            '{"subject_id":"a","raw_text":"รายงาน"}\n', encoding="utf-8"
        )
        result = run_kept(
            "evidence", "rescore", "--offline", "--manifest", str(root / "run-manifest.json"),
            "--raw-jsonl", str(root / "raw.jsonl"), "--output", str(root / "rescore.jsonl")
        )

        self.assertEqual(result.returncode, 0, result.stderr)

    def test_correction_history_appends_without_mutating_raw_jsonl(self) -> None:
        root = fixture_dir("evidence-correct")
        result = run_kept(
            "evidence", "correct", "append", "--history", str(root / "corrections.jsonl"),
            "--raw-jsonl", str(root / "raw.jsonl"), "--subject-id", "subject-1",
            "--decision", "eligible", "--reason", "reviewed"
        )

        self.assertEqual(result.returncode, 0, result.stderr)

    def test_self_test_requires_good_bad_fixtures_and_safety_gate(self) -> None:
        root = fixture_dir("evidence-self-test")
        (root / "good.jsonl").write_text('{"fixture":"good"}\n', encoding="utf-8")
        (root / "bad.jsonl").write_text('{"fixture":"bad"}\n', encoding="utf-8")
        result = run_kept(
            "evidence", "self-test", "--good-fixture", str(root / "good.jsonl"),
            "--bad-fixture", str(root / "bad.jsonl"), "--require-no-mutation"
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("PASS", result.stdout)
        self.assertEqual(
            (root / "good.jsonl").read_text(encoding="utf-8"), '{"fixture":"good"}\n',
            "--require-no-mutation must not overwrite existing fixtures",
        )

    def test_self_test_with_require_no_mutation_fails_on_missing_fixtures(self) -> None:
        root = fixture_dir("evidence-self-test-missing")
        result = run_kept(
            "evidence", "self-test", "--good-fixture", str(root / "good.jsonl"),
            "--bad-fixture", str(root / "bad.jsonl"), "--require-no-mutation"
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("cannot be read without mutation", result.stderr)

    def test_unsafe_run_is_rejected_with_an_explicit_safety_error(self) -> None:
        result = run_kept("evidence", "run", "--allow-mutation")

        self.assertNotEqual(result.returncode, 0)


if __name__ == "__main__":
    unittest.main()
