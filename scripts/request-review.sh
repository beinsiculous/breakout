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
# Reviewer invocation (verified against installed CLIs, Aug 2026):
#   claude: prompt piped on stdin to `claude -p`, response on stdout.
#   kimi:   prompt passed as an argument to `kimi -p <prompt>` (kimi-code
#           0.36+; the old --quiet/--work-dir flags are gone). Progress logs
#           go to stderr, the final review to stdout. There is no work-dir
#           flag anymore, so the invocation cd's into review/ to keep any
#           tool use scoped there. Widening to the repo root would let kimi
#           read surrounding code (better regression context) at the cost of
#           non-interactive tool access — kept conservative on purpose.
#           Linux caps a single argv string at ~128 KiB, so oversized
#           artifacts fail loud with a size check rather than a cryptic
#           "Argument list too long".
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
REVIEW_DIR="$REPO_ROOT/review"
PROMPTS_DIR="$REPO_ROOT/prompts"

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
command -v "$REVIEWER" >/dev/null || { echo "error: '$REVIEWER' not on PATH" >&2; exit 1; }
mkdir -p "$REVIEW_DIR"

if [[ "$MODE" == "plan" ]]; then
    PROMPT_FILE="$PROMPTS_DIR/adversarial-plan-review.md"
    LABEL="PLAN"
else
    PROMPT_FILE="$PROMPTS_DIR/adversarial-code-review.md"
    LABEL="DIFF"
fi

if [[ -z "$OUT" ]]; then
    n=1
    while [[ -e "$REVIEW_DIR/review-$n.md" ]]; do n=$((n+1)); done
    OUT="$REVIEW_DIR/review-$n.md"
fi

echo "==> reviewer ($REVIEWER) critiquing $ARTIFACT -> $OUT" >&2
PAYLOAD="$(
    cat "$PROMPT_FILE"
    printf '\n=== %s UNDER REVIEW ===\n' "$LABEL"
    cat "$ARTIFACT"
)"
case "$REVIEWER" in
    claude) printf '%s\n' "$PAYLOAD" | claude -p > "$OUT" ;;
    kimi)
        # kimi -p takes the prompt as one argv string; Linux caps those at
        # ~128 KiB (MAX_ARG_STRLEN). Fail loud instead of "Argument list too
        # long" — split the artifact or use --reviewer=claude for huge diffs.
        payload_bytes=$(printf '%s' "$PAYLOAD" | wc -c)
        if [[ "$payload_bytes" -gt 120000 ]]; then
            echo "error: payload is $payload_bytes bytes (>120000); too large for kimi -p" >&2
            exit 1
        fi
        (cd "$REVIEW_DIR" && kimi -p "$PAYLOAD") > "$OUT"
        ;;
esac

echo "$OUT"
