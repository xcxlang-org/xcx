#!/usr/bin/env python3
"""
XCX Stability Test Runner
==============================
Runs all .xcx tests and reports results.

Usage:
    python3 runner.py                        # all tests
    python3 runner.py --area database        # single area
    python3 runner.py --priority critical    # single priority
    python3 runner.py --id DB-002            # single test by ID
    python3 runner.py --fail-fast            # stop on first FAIL
    python3 runner.py --verbose              # show full stdout of each test
    python3 runner.py --xcx /path/to/xcx    # custom path to xcx binary
"""

import subprocess
import sys
import os
import re
import time
import argparse
import json
import tempfile
import shutil
from pathlib import Path
from dataclasses import dataclass
from typing import Optional
from enum import Enum
import sys
sys.stdout.reconfigure(encoding="utf-8")

# ─── ANSI Colors ─────────────────────────────────────────────────────────────
NO_COLOR = not sys.stdout.isatty()

def c(color, text):
    if NO_COLOR: return text
    return f"{color}{text}\033[0m"

def bold(t):    return c("\033[1m", t)
def dim(t):     return c("\033[2m", t)
def green(t):   return c("\033[32m", t)
def red(t):     return c("\033[31m", t)
def yellow(t):  return c("\033[33m", t)
def cyan(t):    return c("\033[36m", t)
def magenta(t): return c("\033[35m", t)

# ─── Data Types ──────────────────────────────────────────────────────────────
class Verdict(Enum):
    PASS             = "PASS"
    FAIL             = "FAIL"
    COMPILE_ERROR    = "COMPILE_ERROR"   # expected compilation error — PASS
    FATAL_EXIT       = "FATAL_EXIT"      # expected halt.fatal — PASS
    SKIP             = "SKIP"
    ERROR            = "ERROR"           # runner error
    KNOWN_REGRESSION = "KNOWN_REGRESSION"

VERDICT_LABEL = {
    Verdict.PASS:             green("  PASS  "),
    Verdict.FAIL:             red("  FAIL  "),
    Verdict.COMPILE_ERROR:    green("COMPILE✓"),
    Verdict.FATAL_EXIT:       green(" FATAL✓ "),
    Verdict.SKIP:             yellow("  SKIP  "),
    Verdict.ERROR:            red("  ERR   "),
    Verdict.KNOWN_REGRESSION: yellow("REGRESS "),
}

@dataclass
class TestMeta:
    id:       str = ""
    area:     str = ""
    priority: str = ""
    name:     str = ""
    expect:   str = ""
    expect_compile_error: bool = False
    expect_fatal_exit:    bool = False
    expect_regression:    bool = False
    is_server_test:       bool = False

@dataclass
class TestResult:
    meta:       TestMeta
    path:       Path
    verdict:    Verdict
    duration:   float = 0.0
    stdout:     str = ""
    stderr:     str = ""
    detail:     str = ""
    pass_count: int = 0
    fail_count: int = 0

# ─── Metadata Parser ────────────────────────────────────────────────────────
def parse_meta(path: Path) -> TestMeta:
    meta = TestMeta()
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except Exception:
        return meta

    for line in text.splitlines():
        line = line.strip()
        for key, pat in [
            ("id",       r"^---\s*TEST:\s*(.+)$"),
            ("area",     r"^---\s*Area:\s*(.+)$"),
            ("priority", r"^---\s*Priority:\s*(.+)$"),
            ("name",     r"^---\s*Name:\s*(.+)$"),
            ("expect",   r"^---\s*Expect:\s*(.+)$"),
        ]:
            m = re.match(pat, line)
            if m:
                setattr(meta, key, m.group(1).strip())

    # Flags from runner-hint comments
    meta.expect_compile_error = bool(re.search(
        r"^---\s*Runner:.*expect_compile_error", text, re.MULTILINE | re.IGNORECASE
    )) or ("SHOULD NOT compile" in text)

    meta.expect_fatal_exit = bool(re.search(
        r"^---\s*Runner:.*expect_fatal_exit", text, re.MULTILINE | re.IGNORECASE
    )) or (
        "halt.fatal" in text
        and ("SHOULD NOT execute" in text)
        and not meta.expect_compile_error
    )

    meta.expect_regression = (meta.priority == "regression")
    meta.is_server_test    = bool(re.search(r"^serve\s*:", text, re.MULTILINE))

    return meta

# ─── Compilation Error Detection ─────────────────────────────────────────────
# XCX prints compilation errors to stderr and exits with nonzero process code.
# Looking for patterns clearly indicating a semantic/syntax error.
COMPILE_ERROR_PATTERNS = [
    r"\[S\d+\]",                  # [S208], [S301], [S210], etc.
    r"\[D\d+\]",                  # [D401], etc.
    r"Semantic analysis failed",
    r"Compilation failed",
    r"Syntax error",
    r"Parse error",
]

def looks_like_compile_error(rc: int, stderr: str) -> bool:
    """Returns True if output looks like an XCX COMPILATION error.
    Key: 'Compiled successfully' means the file compiled correctly
    and any ERROR: in stderr is runtime (halt.error), not a compilation error."""
    if rc == 0:
        return False
    # If XCX printed "Compiled successfully" — it was a runtime error, not compile
    if "Compiled successfully" in stderr or "Compiled" in stderr:
        return False
    for pat in COMPILE_ERROR_PATTERNS:
        if re.search(pat, stderr, re.IGNORECASE):
            return True
    return False

def looks_like_runtime_success(rc: int, stderr: str) -> bool:
    """Returns True if xcx compiled and ran the file (even with runtime error)."""
    return "Compiled successfully" in stderr or "Compiled" in stderr

# ─── Running Test ───────────────────────────────────────────────────────
def find_xcx_binary(explicit: Optional[str]) -> Optional[str]:
    if explicit:
        return explicit if (os.path.isfile(explicit) or shutil.which(explicit)) else None
    release_bin = str((Path(__file__).parent.parent.parent / "target" / "release" / "xcx.exe").resolve())
    if os.path.isfile(release_bin):
        return release_bin
    return None

def run_test_file(xcx_bin: str, path: Path, meta: TestMeta,
                  timeout: int, work_dir: Path) -> TestResult:
    result = TestResult(meta=meta, path=path, verdict=Verdict.ERROR)

    # Server tests: skip
    if meta.is_server_test:
        result.verdict = Verdict.SKIP
        result.detail  = "HTTP server test — requires external HTTP client"
        return result

    t0 = time.monotonic()
    try:
        proc = subprocess.run(
            [xcx_bin, str(path)],
            capture_output=True, text=True,
            timeout=timeout, cwd=str(work_dir),
            encoding="utf-8", errors="replace"
        )
        rc, stdout, stderr = proc.returncode, proc.stdout, proc.stderr
    except subprocess.TimeoutExpired:
        result.duration = time.monotonic() - t0
        result.verdict  = Verdict.ERROR
        result.detail   = f"TIMEOUT after {timeout}s"
        return result
    except FileNotFoundError:
        result.duration = time.monotonic() - t0
        result.verdict  = Verdict.ERROR
        result.detail   = f"xcx binary not found: {xcx_bin}"
        return result

    result.duration = time.monotonic() - t0
    result.stdout   = stdout
    result.stderr   = stderr
    result.pass_count, result.fail_count = count_assertions(stdout)

    # ── Expected compilation error ────────────────────────────────
    # Strategy: xcx with compilation error exits nonzero and prints [Sxxx]/[Dxxx] to stderr.
    # Run file normally — if xcx rejects it, stderr contains error codes.
    if meta.expect_compile_error:
        if looks_like_compile_error(rc, stderr):
            result.verdict = Verdict.COMPILE_ERROR
            # Extract error code for information
            codes = re.findall(r"\[[SD]\d+\]", stderr)
            result.detail  = f"Compiler correctly rejected file" + (f": {', '.join(set(codes))}" if codes else "")
        elif rc == 0 or looks_like_runtime_success(rc, stderr):
            result.verdict = Verdict.FAIL
            result.detail  = "FAIL — compiler accepted file that should be rejected"
        else:
            # Nonzero but no recognized patterns — treat as COMPILE_ERROR (xcx might have a different format)
            result.verdict = Verdict.COMPILE_ERROR
            result.detail  = f"Compiler rejected file (exit {rc})"
        return result

    # ── Expected halt.fatal — nonzero exit and no FAIL in stdout ─
    if meta.expect_fatal_exit:
        fail_in_stdout = "] FAIL" in stdout
        if rc != 0 and not fail_in_stdout:
            result.verdict = Verdict.FATAL_EXIT
            result.detail  = f"halt.fatal correctly terminated VM (exit {rc})"
        elif fail_in_stdout:
            result.verdict = Verdict.FAIL
            result.detail  = "FAIL appeared in stdout — halt.fatal did not work"
        else:
            # exit 0 — halt.fatal not called
            result.verdict = Verdict.FAIL
            result.detail  = f"exit code = {rc} — expected nonzero (halt.fatal did not terminate VM)"
        return result

    # ── Regression: known bug ─────────────────────────────────────
    if meta.expect_regression and result.fail_count > 0:
        result.verdict = Verdict.KNOWN_REGRESSION
        result.detail  = "Known regression — FAIL result expected"
        return result

    # ── Standard evaluation based on PASS/FAIL assertions ──────────────────
    if result.fail_count > 0:
        result.verdict = Verdict.FAIL
        result.detail  = f"{result.fail_count} assertions FAIL, {result.pass_count} PASS"
    elif result.pass_count > 0:
        result.verdict = Verdict.PASS
        result.detail  = f"All {result.pass_count} assertions PASS"
    elif rc != 0:
        if looks_like_compile_error(rc, stderr):
            result.verdict = Verdict.FAIL
            result.detail  = "Unexpected compile error (test should not have syntax errors)"
        else:
            result.verdict = Verdict.FAIL
            result.detail  = f"Nonzero exit code ({rc}) without assertions in stdout"
    else:
        result.verdict = Verdict.PASS
        result.detail  = "Exit 0 (no assertions in stdout)"

    return result

def count_assertions(stdout: str) -> tuple[int, int]:
    pass_c = sum(1 for l in stdout.splitlines() if "] PASS" in l)
    fail_c = sum(1 for l in stdout.splitlines() if "] FAIL" in l)
    return pass_c, fail_c

# ─── Collecting tests ─────────────────────────────────────────────────────────
def collect_tests(base_dir: Path, area_filter: str, priority_filter: str,
                  id_filter: str) -> list[tuple[Path, TestMeta]]:
    tests = []
    for xcx_file in sorted(base_dir.rglob("*.xcx")):
        meta = parse_meta(xcx_file)
        if not meta.id:
            meta.id = xcx_file.stem.upper()
        if area_filter     and meta.area.lower()     != area_filter.lower():     continue
        if priority_filter and meta.priority.lower() != priority_filter.lower(): continue
        if id_filter       and meta.id.upper()       != id_filter.upper():       continue
        tests.append((xcx_file, meta))
    return tests

# ─── Formatting ─────────────────────────────────────────────────────────────
PRIORITY_ORDER = {"critical": 0, "high": 1, "medium": 2, "low": 3, "regression": 4}
PRIORITY_COLOR = {"critical": red, "high": yellow, "medium": cyan, "low": dim, "regression": magenta}

def priority_label(p: str) -> str:
    col = PRIORITY_COLOR.get(p, dim)
    return col(f"[{p.upper()[:4]}]")

def print_header(total: int):
    print()
    print(bold("━" * 70))
    print(bold(f"  XCX Stability Test Runner") + dim(f"  ({total} tests)"))
    print(bold("━" * 70))
    print()

def print_section_header(area: str):
    print()
    print(f"  {bold(cyan(area.upper()))}")
    print(f"  {'─'*60}")

def print_result_line(result: TestResult, verbose: bool):
    verdict_str = VERDICT_LABEL.get(result.verdict, result.verdict.value)
    prio    = priority_label(result.meta.priority)
    id_str  = bold(result.meta.id.ljust(10))
    dur     = dim(f"{result.duration*1000:5.0f}ms")
    name    = result.meta.name[:45] + ("…" if len(result.meta.name) > 45 else "")

    print(f"  [{verdict_str}] {prio} {id_str} {dur}  {name}")

    show_detail = verbose or result.verdict in (Verdict.FAIL, Verdict.ERROR)
    if result.detail and show_detail:
        print(f"  {dim('│')}  {dim(result.detail)}")

    if verbose or result.verdict == Verdict.FAIL:
        for line in result.stdout.strip().splitlines()[-8:]:
            tag = green("✓") if "] PASS" in line else (red("✗") if "] FAIL" in line else dim("·"))
            print(f"  {dim('│')}  {tag} {dim(line)}")
        if result.stderr.strip() and result.verdict in (Verdict.FAIL, Verdict.ERROR, Verdict.COMPILE_ERROR):
            for line in result.stderr.strip().splitlines()[-4:]:
                print(f"  {dim('│')}  {yellow('!')} {dim(line)}")

def print_summary(results: list[TestResult], total_time: float):
    counts = {v: 0 for v in Verdict}
    for r in results:
        counts[r.verdict] += 1

    passed  = counts[Verdict.PASS] + counts[Verdict.COMPILE_ERROR] + counts[Verdict.FATAL_EXIT]
    failed  = counts[Verdict.FAIL] + counts[Verdict.ERROR]
    regress = counts[Verdict.KNOWN_REGRESSION]
    skipped = counts[Verdict.SKIP]
    total   = len(results)

    print()
    print(bold("━" * 70))
    print(bold("  SUMMARY"))
    print(bold("━" * 70))
    print()
    print(f"  {bold('Total:')}      {total}")
    print(f"  {green('✓ Passed:')}     {passed}  "
          f"({counts[Verdict.PASS]} run + "
          f"{counts[Verdict.COMPILE_ERROR]} compile✓ + "
          f"{counts[Verdict.FATAL_EXIT]} fatal✓)")
    if failed:
        print(f"  {red('✗ Failed:')}     {failed}")
    if regress:
        print(f"  {yellow('~ Regression:')}  {regress}")
    if skipped:
        print(f"  {dim('- Skipped:')}    {skipped}")
    print(f"  {dim('⏱ Time:')}       {total_time:.2f}s")
    print()

    if failed == 0 and regress == 0:
        print(f"  {bold(green('✓ ALL TESTS PASSED'))}")
    elif failed == 0:
        print(f"  {bold(yellow('~ ONLY KNOWN REGRESSIONS'))}")
    else:
        print(f"  {bold(red('✗ ERRORS FOUND'))}")

    fails = [r for r in results if r.verdict in (Verdict.FAIL, Verdict.ERROR)]
    if fails:
        print()
        print(f"  {bold(red('Failed tests:'))}")
        for r in fails:
            print(f"    {red('✗')} {bold(r.meta.id)} — {r.meta.name}")
            if r.detail:
                print(f"      {dim(r.detail)}")

    print()
    print(bold("━" * 70))
    print()

def save_json_report(results: list[TestResult], output_path: Path):
    report = {
        "runner": "xcx-stability",
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%S"),
        "summary": {
            "total":      len(results),
            "passed":     sum(1 for r in results if r.verdict in (Verdict.PASS, Verdict.COMPILE_ERROR, Verdict.FATAL_EXIT)),
            "failed":     sum(1 for r in results if r.verdict in (Verdict.FAIL, Verdict.ERROR)),
            "regression": sum(1 for r in results if r.verdict == Verdict.KNOWN_REGRESSION),
            "skipped":    sum(1 for r in results if r.verdict == Verdict.SKIP),
        },
        "tests": [
            {
                "id":              r.meta.id,
                "area":            r.meta.area,
                "priority":        r.meta.priority,
                "name":            r.meta.name,
                "verdict":         r.verdict.value,
                "duration_ms":     round(r.duration * 1000),
                "detail":          r.detail,
                "pass_assertions": r.pass_count,
                "fail_assertions": r.fail_count,
                "stdout":          r.stdout[-2000:],
                "stderr":          r.stderr[-1000:],
            }
            for r in results
        ]
    }
    output_path.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"  {dim('→ JSON report:')} {output_path}")

# ─── Main ─────────────────────────────────────────────────────────────────────
def main():
    parser = argparse.ArgumentParser(
        description="XCX Stability Test Runner",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )
    parser.add_argument("--xcx",       metavar="PATH", help="Path to xcx binary")
    parser.add_argument("--area",      metavar="AREA", help="Filter by area")
    parser.add_argument("--priority",  metavar="PRIO", help="Filter by priority")
    parser.add_argument("--id",        metavar="ID",   help="Run single test by ID")
    parser.add_argument("--timeout",   metavar="SEC",  type=int, default=30)
    parser.add_argument("--fail-fast", action="store_true")
    parser.add_argument("--verbose",   action="store_true")
    parser.add_argument("--no-color",  action="store_true")
    parser.add_argument("--json",      metavar="FILE", help="Save JSON report")
    parser.add_argument("--dir",       metavar="DIR",  default=str(Path(__file__).parent))
    args = parser.parse_args()

    global NO_COLOR
    if args.no_color:
        NO_COLOR = True

    base_dir = Path(args.dir).resolve()
    if not base_dir.exists():
        print(red(f"Error: directory '{base_dir}' does not exist"), file=sys.stderr)
        sys.exit(1)

    xcx_bin = find_xcx_binary(args.xcx)
    if not xcx_bin:
        print(red("Error: xcx binary not found."), file=sys.stderr)
        print(dim("Use --xcx /path/to/xcx or add xcx to PATH."), file=sys.stderr)
        sys.exit(1)
    print(dim(f"  xcx binary: {xcx_bin}"))

    tests = collect_tests(base_dir, args.area or "", args.priority or "", args.id or "")
    if not tests:
        print(yellow("No tests found matching criteria."))
        sys.exit(0)

    print_header(len(tests))

    # Working directory with tests_tmp/ subdirectory
    work_dir = Path(tempfile.mkdtemp(prefix="xcx_run_"))
    (work_dir / "tests_tmp").mkdir(exist_ok=True)

    # Group by area, sort by priority
    tests_by_area: dict[str, list] = {}
    for path, meta in tests:
        tests_by_area.setdefault(meta.area or "Other", []).append((path, meta))
    for area in tests_by_area:
        tests_by_area[area].sort(key=lambda t: PRIORITY_ORDER.get(t[1].priority, 99))

    results: list[TestResult] = []
    t_global = time.monotonic()

    try:
        stop = False
        for area, area_tests in tests_by_area.items():
            if stop:
                break
            print_section_header(area)
            for path, meta in area_tests:
                r = run_test_file(xcx_bin, path, meta, args.timeout, work_dir)
                results.append(r)
                print_result_line(r, args.verbose)
                if args.fail_fast and r.verdict in (Verdict.FAIL, Verdict.ERROR):
                    print()
                    print(red("  ✗ --fail-fast: stopping"))
                    stop = True
                    break
    finally:
        shutil.rmtree(work_dir, ignore_errors=True)

    total_time = time.monotonic() - t_global
    print_summary(results, total_time)

    if args.json:
        save_json_report(results, Path(args.json))

    failed = sum(1 for r in results if r.verdict in (Verdict.FAIL, Verdict.ERROR))
    sys.exit(1 if failed > 0 else 0)

if __name__ == "__main__":
    main()
