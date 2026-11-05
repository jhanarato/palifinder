#!/usr/bin/env bash
# Measure line coverage of the workspace's tests with rustc's own instrumentation.
#
# Builds into a separate target directory (COV_TARGET, default /tmp/palifinder-cov-target)
# so the normal target directory is untouched, runs `cargo test --release --workspace`,
# and prints a per-file report. Test code in its own files (tests/, tests.rs) and the
# Snowball-generated files (snowball/pali.rs, snowball_env.rs, among.rs) are left out of
# the figures. Inline `#[cfg(test)] mod tests` blocks inside source files are not: llvm-cov
# filters by file, not by region, so those lines count as covered source. The figures are
# line coverage only, with no branch coverage.
#
# llvm-cov warns that some functions "have mismatched data". A function compiled into more
# than one test binary (the library is built with and without cfg(test)) can get different
# coverage mappings, and llvm-cov then drops one binary's counts for it. That can only
# undercount coverage, never overcount it.
#
# Exits non-zero if src/main.rs or the total falls below 90% of lines.
#
# Needs llvm-profdata and llvm-cov matching rustc's LLVM major version: set LLVM_BIN, or
# they are looked for in Homebrew's llvm, then in rustup's llvm-tools component.
set -euo pipefail

cd "$(dirname "$0")"
COV_TARGET=${COV_TARGET:-/tmp/palifinder-cov-target}
THRESHOLD=${THRESHOLD:-90}

if [[ -z "${LLVM_BIN:-}" ]]; then
    if [[ -x /opt/homebrew/opt/llvm/bin/llvm-cov ]]; then
        LLVM_BIN=/opt/homebrew/opt/llvm/bin
    else
        LLVM_BIN="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/^host: //p')/bin"
    fi
fi
PROFDATA="$LLVM_BIN/llvm-profdata"
LLVMCOV="$LLVM_BIN/llvm-cov"

PROFILES="$COV_TARGET/profiles"
rm -rf "$PROFILES"
mkdir -p "$PROFILES"

export RUSTFLAGS="-C instrument-coverage"
export CARGO_TARGET_DIR="$COV_TARGET"
export LLVM_PROFILE_FILE="$PROFILES/%p-%m.profraw"
cargo test --release --workspace

"$PROFDATA" merge -sparse "$PROFILES"/*.profraw -o "$COV_TARGET/merged.profdata"

# Every test binary and the pali binary itself, which the CLI tests run.
OBJECTS=()
for test_binary in $(cargo test --release --workspace --no-run --message-format=json 2>/dev/null \
        | sed -n 's/.*"executable":"\([^"]*\)".*/\1/p'); do
    OBJECTS+=(-object "$test_binary")
done
if [[ -x "$COV_TARGET/release/pali" ]]; then
    OBJECTS+=(-object "$COV_TARGET/release/pali")
fi

IGNORE='(/\.cargo/|/rustc/|/rustlib/|/tests/|/tests\.rs$|/snowball/(pali|snowball_env|among)\.rs$)'
REPORT=$("$LLVMCOV" report --instr-profile="$COV_TARGET/merged.profdata" \
    --ignore-filename-regex="$IGNORE" "${OBJECTS[@]}")
echo "$REPORT"

# Columns: Filename Regions Missed Cover Functions Missed Executed Lines Missed Cover ...
line_cover() { echo "$REPORT" | awk -v f="$1" '$1 ~ f { print $10 }' | tr -d '%' | head -1; }
main_cover=$(line_cover '(^|/)src/main\.rs$')
total_cover=$(line_cover '^TOTAL$')
echo
echo "src/main.rs lines: ${main_cover}%   total lines: ${total_cover}%   (threshold ${THRESHOLD}%)"
status=0
for pair in "src/main.rs:$main_cover" "total:$total_cover"; do
    name=${pair%%:*}
    value=${pair#*:}
    if [[ -z "$value" ]] || awk -v v="$value" -v t="$THRESHOLD" 'BEGIN { exit !(v < t) }'; then
        echo "FAIL: $name line coverage ${value:-unknown}% is below ${THRESHOLD}%" >&2
        status=1
    fi
done
exit $status
