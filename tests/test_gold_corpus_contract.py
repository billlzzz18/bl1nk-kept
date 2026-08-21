import json
import os
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
default_binary_name = "kept.exe" if os.name == "nt" else "kept"
BINARY = Path(os.environ.get("KEPT_BIN", ROOT / "target" / "debug" / default_binary_name))


def fixture_dir(name: str) -> Path:
    directory = ROOT / "target" / f"tdd-{name}-{os.getpid()}"
    directory.mkdir(parents=True, exist_ok=True)
    return directory


class GoldCorpusContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not BINARY.is_file():
            raise RuntimeError(
                f"kept binary not found at {BINARY}; run cargo build -p kept-cli first"
            )

    def run_kept(self, *arguments: str) -> subprocess.CompletedProcess[str]:
        environment = os.environ.copy()
        environment["NO_COLOR"] = "1"
        return subprocess.run(
            [str(BINARY), *arguments],
            check=False,
            env=environment,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    def test_validation_report_requires_provenance_linked_review_fields_and_target_splits(self) -> None:
        directory = fixture_dir("gold-validate")
        corpus = directory / "gold.jsonl"
        valid_assertion = {
            "source": {"provenanceId": "scan-001", "locator": "page=2#line=4"},
            "context": {"fragmentHash": "a" * 64},
            "candidate": "รายงาน",
            "normalizedCandidate": "รายงาน",
            "assertionKind": "canonical",
            "review": {"decision": "accepted", "reason": "explicit source match"},
            "split": "build",
        }

        for missing_field in (
            "source.provenanceId",
            "source.locator",
            "context.fragmentHash",
            "normalizedCandidate",
            "assertionKind",
            "review.decision",
            "review.reason",
            "split",
        ):
            invalid_assertion = json.loads(json.dumps(valid_assertion))
            owner, _, field = missing_field.partition(".")
            if owner in ("source", "context", "review"):
                invalid_assertion[owner].pop(field)
            else:
                invalid_assertion.pop(missing_field)
            corpus.write_text(json.dumps(invalid_assertion, ensure_ascii=False) + "\n", encoding="utf-8")

            with self.subTest(missing_field=missing_field):
                result = self.run_kept("corpus", "validate", str(corpus), "--report", "json")
                self.assertEqual(result.returncode, 0, result.stderr)
                report = json.loads(result.stdout)
                self.assertIn(missing_field, report["errors"][0]["missing"])
                self.assertEqual(report["targetSplit"], {"build": 7000, "validation": 1500, "holdout": 1500})

    def test_snapshot_round_trip_and_replay_are_deterministic_without_promoting_hypotheses(self) -> None:
        directory = fixture_dir("gold-replay")
        corpus = directory / "reviewed.jsonl"
        snapshot = directory / "gold.snapshot.json"
        corpus.write_text(
            json.dumps(
                {
                    "source": {"provenanceId": "md-001", "locator": "heading=1"},
                    "context": {"fragmentHash": "b" * 64},
                    "candidate": "เอกสาร",
                    "normalizedCandidate": "เอกสาร",
                    "assertionKind": "canonical",
                    "review": {"decision": "accepted", "reason": "reviewed"},
                    "split": "build",
                }, ensure_ascii=False
            )
            + "\n"
            + json.dumps(
                {
                    "source": {"provenanceId": "md-001", "locator": "heading=2"},
                    "context": {"fragmentHash": "c" * 64},
                    "candidate": "เอกสาน",
                    "normalizedCandidate": "เอกสาร",
                    "assertionKind": "fuzzy",
                    "review": {"decision": "rejected", "reason": "candidate only; not accepted"},
                    "split": "build",
                }, ensure_ascii=False
            )
            + "\n",
            encoding="utf-8",
        )

        saved = self.run_kept("corpus", "snapshot", "save", str(corpus), str(snapshot))
        self.assertEqual(saved.returncode, 0, saved.stderr)
        first = self.run_kept("corpus", "replay", str(snapshot), "--json")
        second = self.run_kept("corpus", "replay", str(snapshot), "--json")

        self.assertEqual(first.returncode, 0, first.stderr)
        self.assertEqual(second.returncode, 0, second.stderr)
        self.assertEqual(json.loads(first.stdout), json.loads(second.stdout))
        self.assertEqual(json.loads(first.stdout)["dictionary"], ["เอกสาร"])
        self.assertNotIn("เอกสาน", json.loads(first.stdout)["dictionary"])


if __name__ == "__main__":
    unittest.main()
