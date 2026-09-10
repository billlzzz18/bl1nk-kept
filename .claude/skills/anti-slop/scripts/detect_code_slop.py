#!/usr/bin/env python3
"""
Rust Code Slop Detector - Uses tree-sitter to detect AI-generated code patterns.
"""
import re
import sys
from pathlib import Path
from typing import List, Dict, Optional

try:
    import tree_sitter_rust as tsrust
    from tree_sitter import Language, Parser
    HAS_TREE_SITTER = True
except ImportError:
    HAS_TREE_SITTER = False


# Obvious comment patterns — comments that just restate what code does
OBVIOUS_COMMENT_PATTERNS = [
    r"//\s*(create|initialize|set up|get|return|print|loop|iterate|check|if|else|match)\b",
    r"//\s*(this function|this method|this struct|this enum|this trait)\b",
    r"//\s*(adds?|removes?|updates?|deletes?|creates?|returns?)\s+the\b",
    r"//\s*constructor",
    r"//\s*getter",
    r"//\s*setter",
    r"//\s*TODO:?\s*$",  # empty TODO
    r"//\s*FIXME:?\s*$",
]

# Generic variable names that signal lazy AI naming
GENERIC_NAMES = [
    r"\b(data|result|output|input|item|element|value|temp|tmp|ret|res)\b",
    r"\b(helper|handler|manager|processor|wrapper|container)\b",
    r"\b(info|details|status|context|state|config|options|params|args)\b",
]

# Overly verbose patterns
VERBOSE_PATTERNS = [
    # Redundant type annotations where compiler can infer
    (r"let\s+\w+\s*:\s*(?:Vec|HashMap|String|Option|Result|Box|Rc|Arc|Cell|RefCell)\s*=", "redundant_type_annotation"),
    # Unnecessary clone in non-async contexts
    (r"\.clone\(\)\s*;", "unnecessary_clone"),
    # Double negation or redundant bool checks
    (r"if\s+\w+\s*==\s*true\b", "redundant_bool_check"),
    (r"if\s+\w+\s*==\s*false\b", "redundant_bool_check"),
    (r"if\s+!\w+\s*==\s*false\b", "redundant_bool_check"),
    # Empty catch/unwrap chains
    (r"\.unwrap\(\)\s*;", "unwrap_in_production"),
]

# Over-abstraction indicators
ABSTRACTION_PATTERNS = [
    # Interface/trait with only one implementor (detected via structure)
    r"impl\s+\w+\s+for\s+\w+",
]


class RustCodeSlopDetector:
    def __init__(self, filepath: str):
        self.filepath = Path(filepath)
        self.lines = self.filepath.read_text(encoding="utf-8").splitlines()
        self.findings: Dict[str, List[Dict]] = {
            "obvious_comments": [],
            "generic_names": [],
            "verbose_code": [],
            "empty_impls": [],
            "structure": [],
        }

    def analyze(self) -> Dict:
        """Run all analyses."""
        self._check_comments()
        self._check_verbose_patterns()
        if HAS_TREE_SITTER:
            self._analyze_ast()
        self._check_structure()
        return {
            "findings": self.findings,
            "score": self._calculate_score(),
            "summary": self._generate_summary(),
        }

    def _check_comments(self):
        """Detect obvious/restating comments. Skip doc comments (///)."""
        for i, line in enumerate(self.lines, 1):
            stripped = line.strip()
            if not stripped.startswith("//"):
                continue
            # Skip doc comments (///) and module-level docs (//!)
            if stripped.startswith("///") or stripped.startswith("//!"):
                continue
            comment = stripped.lstrip("/").strip()
            if not comment:
                continue
            for pattern in OBVIOUS_COMMENT_PATTERNS:
                if re.search(pattern, stripped, re.IGNORECASE):
                    self.findings["obvious_comments"].append({
                        "line": i,
                        "text": stripped,
                        "pattern": pattern,
                        "confidence": "high",
                        "savings": "1 line, 0 logic change",
                        "action": "delete",
                    })
                    break

    def _check_verbose_patterns(self):
        """Detect verbose/unnecessary code patterns."""
        SAVINGS = {
            "redundant_type_annotation": "1 line, let inference work",
            "unnecessary_clone": "1 line, remove clone if ownership allows",
            "redundant_bool_check": "1 line, simplify boolean expression",
            "unwrap_in_production": "1 line, replace with ? or expect()",
            "unused_binding": "1 line, remove unused binding",
        }
        CONFIDENCE = {
            "redundant_type_annotation": "high",
            "unnecessary_clone": "medium",
            "redundant_bool_check": "high",
            "unwrap_in_production": "high",
            "unused_binding": "medium",
        }
        for i, line in enumerate(self.lines, 1):
            for pattern, category in VERBOSE_PATTERNS:
                if re.search(pattern, line):
                    self.findings["verbose_code"].append({
                        "line": i,
                        "text": line.strip(),
                        "category": category,
                        "confidence": CONFIDENCE.get(category, "medium"),
                        "savings": SAVINGS.get(category, "1 line"),
                        "action": "review" if CONFIDENCE.get(category) == "medium" else "delete",
                    })

    def _analyze_ast(self):
        """Use tree-sitter for deeper analysis."""
        try:
            lang = Language(tsrust.language())
            parser = Parser(lang)
            code = self.filepath.read_text(encoding="utf-8")
            tree = parser.parse(bytes(code, "utf8"))

            self._check_function_lengths(tree.root_node)
            self._check_nested_depth(tree.root_node)
            self._check_impl_block_count(tree.root_node)
        except Exception:
            pass  # tree-sitter not available or parse error

    def _check_function_lengths(self, node, depth=0):
        """Flag overly long functions (>80 lines)."""
        if node.type == "function_item":
            start = node.start_point[0]
            end = node.end_point[0]
            length = end - start
            if length > 80:
                # Get function name
                name_node = None
                for child in node.children:
                    if child.type == "identifier":
                        name_node = child
                        break
                name = name_node.text.decode() if name_node else "<unknown>"
                savings = max(1, length - 40)  # assume can extract to ~40 lines
                self.findings["structure"].append({
                    "issue": "long_function",
                    "line": start + 1,
                    "detail": f"Function '{name}' is {length} lines (>80)",
                    "confidence": "medium",
                    "savings": f"~{savings} lines if extracted into sub-functions",
                    "action": "review",
                })
        for child in node.children:
            self._check_function_lengths(child, depth + 1)

    def _check_nested_depth(self, node, depth=0, max_depth=0):
        """Flag deep nesting (>4 levels). Only count control-flow nesting, not every block."""
        if node.type in ("if_expression", "match_arm", "loop_expression", "while_expression"):
            depth += 1
            if depth > 4:
                self.findings["structure"].append({
                    "issue": "deep_nesting",
                    "line": node.start_point[0] + 1,
                    "detail": f"Nesting depth {depth} (>4) at line {node.start_point[0] + 1}",
                    "confidence": "medium",
                    "savings": "~5-10 lines with early returns",
                    "action": "review",
                })
        for child in node.children:
            self._check_nested_depth(child, depth, max(depth, max_depth))

    def _check_impl_block_count(self, node):
        """Flag excessive impl blocks (potential over-abstraction)."""
        impl_count = 0
        trait_impls = {}

        def count_impls(n):
            nonlocal impl_count
            if n.type == "impl_item":
                impl_count += 1
                # Check if it's a trait impl
                for child in n.children:
                    if child.type == "type_identifier":
                        trait_name = child.text.decode()
                        trait_impls.setdefault(trait_name, 0)
                        trait_impls[trait_name] += 1
            for c in n.children:
                count_impls(c)

        count_impls(node)

        if impl_count > 10:
            self.findings["structure"].append({
                "issue": "many_impl_blocks",
                "detail": f"{impl_count} impl blocks — consider consolidating",
                "confidence": "low",
                "savings": "varies — review each impl",
                "action": "info",
            })

        # Flag traits with only 1 implementor (potential over-abstraction)
        for trait_name, count in trait_impls.items():
            if count == 1 and trait_name[0].isupper():
                self.findings["structure"].append({
                    "issue": "single_impl_trait",
                    "detail": f"Trait '{trait_name}' has only 1 impl — possibly over-abstracted",
                    "confidence": "medium",
                    "savings": "~5-15 lines if trait removed",
                    "action": "review",
                })

    def _check_structure(self):
        """File-level structural checks."""
        total_lines = len(self.lines)
        comment_lines = sum(1 for l in self.lines if l.strip().startswith("//"))
        code_lines = total_lines - comment_lines

        if code_lines > 0:
            comment_ratio = comment_lines / code_lines
            if comment_ratio > 0.5:
                excess = comment_lines - code_lines
                self.findings["structure"].append({
                    "issue": "excessive_comments",
                    "detail": f"Comment ratio {comment_ratio:.0%} (>50%) — too many comments",
                    "confidence": "medium",
                    "savings": f"~{excess} lines if obvious comments removed",
                    "action": "review",
                })

        # Check for dead code patterns
        for i, line in enumerate(self.lines, 1):
            stripped = line.strip()
            if re.match(r"let\s+_\w+\s*=", stripped):
                self.findings["verbose_code"].append({
                    "line": i,
                    "text": stripped,
                    "category": "unused_binding",
                })

    def _calculate_score(self) -> int:
        score = 0
        score += len(self.findings["obvious_comments"]) * 12
        score += len(self.findings["generic_names"]) * 3
        score += len(self.findings["verbose_code"]) * 8
        for f in self.findings["structure"]:
            if f["issue"] == "long_function":
                score += 15
            elif f["issue"] == "deep_nesting":
                score += 10
            elif f["issue"] == "excessive_comments":
                score += 20
            elif f["issue"] == "single_impl_trait":
                score += 10
            elif f["issue"] == "many_impl_blocks":
                score += 10

        # Normalize by line count
        if len(self.lines) > 0:
            score = int((score / len(self.lines)) * 100)
        return min(score, 100)

    def _generate_summary(self) -> str:
        score = self._calculate_score()
        if score < 15:
            return "✅ Clean — code appears intentional and well-structured"
        elif score < 30:
            return "⚠️  Moderate slop — some AI-generated patterns detected"
        elif score < 50:
            return "🚨 High slop — many generic patterns found"
        else:
            return "💀 Severe slop — code heavily relies on AI patterns"

    def print_report(self, verbose: bool = False):
        results = self.analyze()

        print(f"\n{'='*70}")
        print(f"Rust Code Slop Detection: {self.filepath.name}")
        print(f"{'='*70}\n")
        print(f"Score: {results['score']}/100 — {results['summary']}\n")

        f = results["findings"]

        # Collect all findings with confidence levels
        definite = []  # high confidence
        suspicious = []  # medium confidence
        info = []  # low confidence

        for c in f.get("obvious_comments", []):
            entry = {"type": "obvious_comment", **c}
            if c.get("confidence") == "high":
                definite.append(entry)
            else:
                suspicious.append(entry)

        for v in f.get("verbose_code", []):
            entry = {"type": "verbose_code", **v}
            if v.get("confidence") == "high":
                definite.append(entry)
            else:
                suspicious.append(entry)

        for s in f.get("structure", []):
            entry = {"type": "structure", **s}
            conf = s.get("confidence", "low")
            if conf == "high":
                definite.append(entry)
            elif conf == "medium":
                suspicious.append(entry)
            else:
                info.append(entry)

        # Report definite slop
        if definite:
            print(f"🔴 DEFINITE SLOP ({len(definite)} found):")
            for d in definite[:10 if not verbose else None]:
                loc = f"Line {d['line']}" if "line" in d else ""
                print(f"  {loc}: {d.get('detail', d.get('text', ''))[:60]}")
                print(f"    Savings: {d.get('savings', '1 line')}")
                print(f"    Action: {d.get('action', 'delete')}")
            print()

        # Report suspicious slop
        if suspicious:
            print(f"🟡 SUSPICIOUS ({len(suspicious)} found):")
            for s in suspicious[:10 if not verbose else None]:
                loc = f"Line {s['line']}" if "line" in s else ""
                print(f"  {loc}: {s.get('detail', s.get('text', ''))[:60]}")
                print(f"    Savings: {s.get('savings', 'varies')}")
                print(f"    Recommendation: review — context may justify this")
            print()

        # Report info
        if info:
            print(f"ℹ️  INFO ({len(info)} found):")
            for i in info:
                print(f"  • {i.get('detail', '')}")
            print()

        # Summary
        if results["score"] > 15:
            total_savings = len(definite) + len(suspicious)
            print(f"📊 Summary: {len(definite)} definite + {len(suspicious)} suspicious = {total_savings} items to review")
            print(f"   Auto-fixable: {len(definite)} items (high confidence)")


def main():
    if len(sys.argv) < 2:
        print("Usage: python detect_code_slop.py <file.rs> [--verbose]")
        print("Detects AI-generated code patterns in Rust files")
        sys.exit(1)

    filepath = sys.argv[1]
    verbose = "--verbose" in sys.argv or "-v" in sys.argv

    if not Path(filepath).exists():
        print(f"Error: '{filepath}' not found")
        sys.exit(1)

    detector = RustCodeSlopDetector(filepath)
    detector.print_report(verbose=verbose)


if __name__ == "__main__":
    main()
