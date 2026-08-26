#!/usr/bin/env bash
# One-shot adversarial review of an artifact by a headless counterpart CLI.
# Used by the interactive skill (.claude/skills/adversarial-review/) and by
# the fully-headless driver (adversarial-review.sh).
#
#   request-review.sh plan review/plan.md   --reviewer=kimi
#   request-review.sh code review/draft.diff --reviewer=claude [--out=path]
#
# Writes the review to --out (default: review/review-N.md, N auto-incremented)
# and prints the output path on stdout so callers can capture it.
#
# How the reviewer is actually invoked lives in scripts/lib/headless-agent.sh, which this and
# adversarial-review.sh both source — they used to spell it out separately and drifted apart.
# A kimi reviewer runs in review/, so its tool use is confined there and it cannot read the
# surrounding code — which is why the prompt carries the whole artifact. A claude reviewer is not
# confined; see the note in lib/headless-agent.sh.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
REVIEW_DIR="$REPO_ROOT/review"
PROMPTS_DIR="$REPO_ROOT/prompts"
# shellcheck source=lib/headless-agent.sh
. "$SCRIPT_DIR/lib/headless-agent.sh"

usage() {
    cat >&2 <<EOF
Usage: $(basename "$0") <plan|code> <artifact-path> --reviewer=claude|kimi [--out=path]

  plan mode: artifact is a plan document (uses prompts/adversarial-plan-review.md)
  code mode: artifact is a diff        (uses prompts/adversarial-code-review.md)
  --reviewer  which CLI critiques the artifact
  --out       output file (default: review/review-N.md, N auto-incremented)
EOF
    exit 1
}

MODE="${1:-}"; ARTIFACT="${2:-}"; REVIEWER=""; OUT=""
shift 2 2>/dev/null || usage
for arg in "$@"; do
    case "$arg" in
        --reviewer=claude|--reviewer=kimi) REVIEWER="${arg#--reviewer=}" ;;
        --out=*) OUT="${arg#--out=}" ;;
        *) echo "error: unknown argument '$arg'" >&2; usage ;;
    esac
done

[[ "$MODE" == "plan" || "$MODE" == "code" ]] || usage
[[ -n "$REVIEWER" ]] || usage
[[ -f "$ARTIFACT" ]] || { echo "error: artifact not found: $ARTIFACT" >&2; exit 1; }
if [[ "$REVIEWER" == "kimi" ]]; then
    [[ -n "$(headless_kimi_binary)" ]] || { echo "error: kimi not on PATH (set KIMI_BIN)" >&2; exit 1; }
else
    command -v "$REVIEWER" >/dev/null || { echo "error: '$REVIEWER' not on PATH" >&2; exit 1; }
fi
mkdir -p "$REVIEW_DIR"

if [[ "$MODE" == "plan" ]]; then
    PROMPT_FILE="$PROMPTS_DIR/adversarial-plan-review.md"
    LABEL="PLAN"
else
    PROMPT_FILE="$PROMPTS_DIR/adversarial-code-review.md"
    LABEL="DIFF"
fi

# Claim the name by CREATING it, not by looking and then writing. Two sessions reviewing at
# once — which is the intended setup, one per harness — both saw the same lowest free N and the
# second overwrote the first, leaving one of them adjudicating a review of someone else's diff.
# noclobber makes the create-or-fail atomic.
if [[ -z "$OUT" ]]; then
    n=1
    while true; do
        candidate="$REVIEW_DIR/review-$n.md"
        if (set -o noclobber; : > "$candidate") 2>/dev/null; then OUT="$candidate"; break; fi
        n=$((n+1))
        if [[ "$n" -gt 999 ]]; then
            echo "error: review/ already holds 999 reviews; clear the settled ones" >&2
            exit 1
        fi
    done
fi

echo "==> reviewer ($REVIEWER) critiquing $ARTIFACT -> $OUT" >&2

# The prompt goes to a file rather than a variable: past a certain size it can no longer be
# passed to kimi as an argv argument, and the library needs it on disk to hand over instead.
PROMPT_TMP="$REVIEW_DIR/.request-prompt.$$"
trap 'rm -f "$PROMPT_TMP"' EXIT
{
    cat "$PROMPT_FILE"
    printf '\n=== %s UNDER REVIEW ===\n' "$LABEL"
    cat "$ARTIFACT"
} > "$PROMPT_TMP"

# The name above was claimed by creating the file, so a reviewer that dies leaves an empty
# review behind and burns that N for good. An empty one is cleared; a partial one is kept and
# said out loud, because a truncated review must never be mistaken for a settled one.
# `if ! cmd; then ... $?` reads the status of the NEGATION, which is 0 exactly when cmd failed —
# so this used to exit 0 on every reviewer failure, telling the gate and the skill that a review
# had succeeded while writing none. Fail-open at the process level, at the one point that exists
# to make failure visible. The status is captured in the else branch, where it is the real one.
if run_headless_agent "$REVIEWER" "$PROMPT_TMP" "$REVIEW_DIR" > "$OUT"; then
    :
else
    status=$?
    if [[ -s "$OUT" ]]; then
        echo "error: $REVIEWER failed after writing $(wc -c < "$OUT") bytes; $OUT is INCOMPLETE" >&2
    else
        rm -f "$OUT"
        echo "error: $REVIEWER produced nothing; no review was written" >&2
    fi
    exit "$status"
fi

echo "$OUT"
