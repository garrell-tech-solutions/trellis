"""Shared helpers for the Rust analyzer wrappers (complexity.sh, crap.sh).

Not an Analyzer Contract entry point itself -- imported by the scripts that
are.
"""
import os
import re
import subprocess


def rust_files_under(path, root):
    """Resolve a file-or-directory analyzer argument to a list of source
    files, excluding build output, git worktrees, and generated
    acceptance-test entry points (see .gitignore: those are regenerated
    from features/*.feature and are not hand-maintained source).

    `find` does not read .gitignore, so `.claude/worktrees/agent-*/` and
    `.worktrees/<role>/` -- each a whole second copy of this tree -- are
    walked unless excluded here, and every function in them is counted
    again under a path nobody is working in.

    Exclusions are matched against the path *relative to `root`*, not the
    path as `find` printed it. Matching the raw path would mean an analyzer
    invoked from inside a worktree with an absolute argument saw
    `/.claude/` in every result and reported a clean, empty run."""
    out = subprocess.run(
        ["find", path, "-name", "*.rs", "-type", "f"],
        capture_output=True, text=True, check=True,
    ).stdout.splitlines()
    skip_substrings = (
        "/target/", "/build/", "/mutants.out", "/.worktrees/", "/.claude/",
    )
    root_abs = os.path.abspath(root)
    files = []
    for f in out:
        rel = "/" + os.path.relpath(os.path.abspath(f), root_abs)
        if any(s in rel for s in skip_substrings):
            continue
        if re.search(r"/tests/[^/]*_acceptance\.rs$", rel):
            continue
        files.append(f)
    return sorted(files)


def cfg_test_ranges(source_text):
    """Line ranges (1-indexed, inclusive) covered by `#[cfg(test)] mod ... {
    ... }` blocks, found by brace counting from the mod's opening brace."""
    lines = source_text.splitlines()
    ranges = []
    i = 0
    while i < len(lines):
        if re.match(r"\s*#\[cfg\(test\)\]\s*$", lines[i]):
            j = i + 1
            while j < len(lines) and lines[j].strip() == "":
                j += 1
            if j < len(lines) and re.search(r"\bmod\s+\w+\s*\{", lines[j]):
                depth = lines[j].count("{") - lines[j].count("}")
                start = j + 1
                k = j + 1
                while k < len(lines) and depth > 0:
                    depth += lines[k].count("{") - lines[k].count("}")
                    k += 1
                ranges.append((start, k))
                i = k
                continue
        i += 1
    return ranges


def in_ranges(line, ranges):
    return any(lo <= line <= hi for lo, hi in ranges)


def iter_named_functions(space, depth=0):
    """Recursively yield kind=="function" spaces with a real name (skips
    closures, which rust-code-analysis-cli reports as name "<anonymous>")."""
    if space.get("kind") == "function" and space.get("name") != "<anonymous>":
        yield space
    for child in space.get("spaces", []):
        yield from iter_named_functions(child, depth + 1)


def production_functions(rca_unit, source_text):
    """Functions in `rca_unit` (one rust-code-analysis-cli --metrics JSON
    object) that are not test code, as (name, start_line, end_line,
    cyclomatic) tuples."""
    ranges = cfg_test_ranges(source_text)
    result = []
    for fn in iter_named_functions(rca_unit):
        start = fn["start_line"]
        if in_ranges(start, ranges):
            continue
        cyclomatic = fn["metrics"]["cyclomatic"]["sum"]
        result.append((fn["name"], start, fn["end_line"], cyclomatic))
    return result
