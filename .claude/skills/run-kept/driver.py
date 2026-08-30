from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[3]
KEPT = ROOT / "target" / "debug" / "kept.exe"


def run(*args):
    result = subprocess.run([str(KEPT), *args], cwd=ROOT, text=True, capture_output=True)
    if result.returncode:
        raise SystemExit(f"kept {' '.join(args)} failed: {result.stderr.strip()}")
    return result.stdout


subprocess.run(["cargo", "build", "-q", "-p", "kept-cli", "--bin", "kept"], cwd=ROOT, check=True)
assert "Usage:" in run("--help")
fixture = Path(tempfile.mkdtemp(prefix="kept-run-"))
try:
    (fixture / "note.md").write_text("# kept smoke\n", encoding="utf-8")
    assert "Indexed" in run("scan", str(fixture))
    assert "note.md" in run("find", str(fixture), "--name", "note")
    output = fixture / "copy.md"
    run("convert", str(fixture / "note.md"), str(output))
    assert output.is_file()
finally:
    shutil.rmtree(fixture, ignore_errors=True)
print("run-kept smoke passed")
