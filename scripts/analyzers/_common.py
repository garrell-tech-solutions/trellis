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

    `tmp/` is excluded for a related reason: the constitution tells every
    agent to put scratch files there ("Use ./tmp/ in your assigned worktree
    for temporary files"), so a copy of a source file made while debugging
    is normal and must not be able to fail a gate on its own.
    scripts/analyzers/dry.sh has ignored it since it was written; this is
    the same exclusion for the analyzers that walk the tree themselves.

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
        "/tmp/",
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


# ---------------------------------------------------------------------------
# ONE rust-code-analysis-cli walk, shared.
#
# complexity.sh and crap.sh both need the same thing -- every production
# function in the tree with its cyclomatic complexity -- and each used to walk
# the tree itself, spawning rust-code-analysis-cli once per file. In CI both
# run, back to back, over an identical file list, so the whole walk happened
# twice for one answer.
#
# Correctness comes first here: an analyzer that reuses a stale measurement
# reports a number about a file that has since changed, which is worse than
# reporting nothing. So each entry is fingerprinted with the file's size and
# nanosecond mtime -- the same pair Cargo trusts to decide whether a source
# file needs rebuilding -- and any mismatch re-measures that file. The cache is
# per file, so an edit invalidates that file and nothing else.
#
# It is also strictly an optimisation. A missing, unreadable or corrupt cache
# is a miss, not an error; every analyzer keeps working standalone with nothing
# else having run first, which the Analyzer Contract requires. The cache lives
# under the cargo target directory, which is gitignored and which
# rust_files_under() excludes, so it can never become something the analyzers
# measure.
_WALK_CACHE_PATH = os.path.join(
    os.environ.get("CARGO_TARGET_DIR", "target"),
    "analyzers",
    "rust-code-analysis-walk.json",
)


def _walk_fingerprint(path):
    st = os.stat(path)
    return [st.st_size, st.st_mtime_ns]


def _measure_file(path):
    import json
    raw = subprocess.run(
        ["rust-code-analysis-cli", "-p", path, "-m", "-O", "json"],
        capture_output=True, text=True, check=True,
    ).stdout
    with open(path) as fh:
        source = fh.read()
    found = []
    for line in raw.splitlines():
        if not line.strip():
            continue
        found.extend(production_functions(json.loads(line), source))
    return found


def production_function_index(files, cache_path=_WALK_CACHE_PATH):
    """{file: [(name, start_line, end_line, cyclomatic), ...]} for `files`,
    measured with rust-code-analysis-cli and cached per file by size+mtime."""
    import json

    try:
        with open(cache_path) as fh:
            cache = json.load(fh)
        if not isinstance(cache, dict):
            cache = {}
    except (OSError, ValueError):
        cache = {}

    index = {}
    changed = False
    for f in files:
        key = os.path.abspath(f)
        fingerprint = _walk_fingerprint(f)
        cached = cache.get(key)
        if isinstance(cached, dict) and cached.get("fingerprint") == fingerprint:
            index[f] = [tuple(fn) for fn in cached.get("functions", [])]
            continue
        functions = _measure_file(f)
        index[f] = functions
        cache[key] = {"fingerprint": fingerprint,
                      "functions": [list(fn) for fn in functions]}
        changed = True

    if changed:
        try:
            os.makedirs(os.path.dirname(cache_path) or ".", exist_ok=True)
            tmp = cache_path + ".tmp"
            with open(tmp, "w") as fh:
                json.dump(cache, fh)
            os.replace(tmp, cache_path)
        except OSError:
            pass  # an uncacheable run is a slow run, not a wrong one

    return index
