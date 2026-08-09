#!/usr/bin/env bash
# PreToolUse hook (Bash matcher) for Claude Code: before a `git commit`, if the
# pending diff is big, inject a reminder that the project convention is an
# adversarial code review (scripts/request-review.sh code ...) before committing.
#
# Silent (exit 0, no output) for: non-commit commands, non-repo dirs, diffs
# under THRESHOLD changed lines, and commits prefixed with ADV_REVIEWED=1
# (set by the agent only after a code-mode review was adjudicated with the
# user, or the user explicitly skipped review). Otherwise DENIES the commit
# with instructions — informing alone can't stop the triggering command.
set -euo pipefail

THRESHOLD=100

# Degrade gracefully where jq is missing (review-3 F1): a reminder gate must
# never break basic shell use on a machine without the dependency.
command -v jq >/dev/null 2>&1 || exit 0

input=$(cat)
cmd=$(printf '%s' "$input" | jq -r '.tool_input.command // ""')
# All matching runs against the command with quoted segments removed, so
# commit-message text can neither smuggle the ADV_REVIEWED=1 bypass
# (deion_assets review-1 F3) nor trigger the staging fallback below
# (review-1 F2).
stripped=$(printf '%s' "$cmd" | sed "s/'[^']*'//g; s/\"[^\"]*\"//g")
case "$stripped" in
    *"git commit"*) ;;
    *) exit 0 ;;
esac
# Bypass token counts only as a leading env assignment (or one right after
# && / ;) — never as free text inside a commit message (review-3 F3).
case "$stripped" in
    "ADV_REVIEWED=1 "* | *"&& ADV_REVIEWED=1 "* | *"; ADV_REVIEWED=1 "*) exit 0 ;;
esac

top="$(git rev-parse --show-toplevel 2>/dev/null)" || exit 0
cd "$top"

changed_lines() {
    # numstat: insertions<TAB>deletions<TAB>path; "-" for binary counts as 0
    git diff ${1:-} --numstat 2>/dev/null | awk '{a += $1 + $2} END {print a + 0}'
}

lines=$(changed_lines --cached)
# When this command also stages (`commit -a` or a `git add` in the same
# compound), the staged count alone undercounts — a small pre-staged change
# would mask a large working tree (deion_assets review-1 F1). Take the larger
# of the two. Without such staging, a pathspec/--only commit with unrelated
# local changes must not be gated by working-tree size (review-3 F2) — if we
# can't size what is being committed, pass silently.
case "$stripped" in
    *"git add"* | *" -a"* | *"--all"*)
        wt=$(changed_lines)
        [ "${wt:-0}" -gt "${lines:-0}" ] && lines=$wt
        ;;
esac
[ "${lines:-0}" -eq 0 ] && exit 0
[ "${lines:-0}" -lt "$THRESHOLD" ] && exit 0

jq -n --arg n "$lines" --arg t "$THRESHOLD" '{hookSpecificOutput: {hookEventName: "PreToolUse", permissionDecision: "deny", permissionDecisionReason: ("Blocked by project convention: big commits get an adversarial CODE review before landing. The pending diff is " + $n + " changed lines (threshold " + $t + "). Write the diff (git diff --cached > review/draft.diff; use git diff if staging happens in the same command), run scripts/request-review.sh code review/draft.diff --reviewer=kimi, present and adjudicate every finding with the user, apply accepted fixes — then retry the commit with the command prefixed ADV_REVIEWED=1 (also use that prefix if the user explicitly skips review, or if this exact diff was already reviewed this session). Clear review/ artifacts first ONLY if they belong to a previous, settled review subject.")}}'
