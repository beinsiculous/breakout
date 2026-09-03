# Handoff — execute one batch of a settled plan

You are executing **one batch** of a plan another agent wrote and two reviewers
have settled. You do not change the plan; where it is wrong or impossible, you
stop and say so in your report instead of improvising.

## Read first, in this order

1. `<PLAN_PATH>` § "Ground rules for every batch", then § "<BATCH_SECTION>" —
   the batch you are executing, and nothing else in that file is your scope.
2. `<DESIGN_PATH>` § <DESIGN_SECTIONS> for the exact target shapes (types,
   signatures, files). Do not re-derive them.
3. The repo guide (`CLAUDE.md`) § "Solo Session Guardrails" and "Known
   Footguns", and `training.md` § "Writing Tests" for any test you write.

## Rules

- Branch `<BRANCH>`, in this checkout. Another session owns the tree between
  batches; touch only the files the batch names plus what a compile forces.
- **Stage everything you touched — new files included (`git add` each path;
  an unstaged new file is invisible to the reviewers but present for your
  gates, so a green run can commit a tree that does not compile). Do not
  commit.** The planner verifies, two reviewers read the staged diff, and the
  planner commits.
- If you must stop before the batch is complete (an impossibility, an error
  you cannot resolve, running out of time or credits), stage what you have
  and send the report anyway, marked INCOMPLETE, so the tree is never left
  with edits nobody knows about.
- Every deletion of a public item shows its grep in your report:
  `grep -rn "<symbol>" crates src examples ../games --include=*.rs | grep -v target/`.
  If the symbol is live, keep it and say so with the grep.
- Before deleting the only test of a public API, the same grep; if the API
  is live and no other test covers it, keep one strengthened, contract-named
  test.
- No `#[allow]`, no `unwrap()` outside test code, every touched file ≤ 600
  lines, no new dependencies, no comments that restate the next line.
- Doc lines the batch names are part of the batch and land in the same
  change; a guide that describes a deleted thing is a defect.

## Gates before you report (all must be clean)

```
cargo test --workspace                          # 0 failed, 0 ignored
cargo clippy --workspace --all-targets          # 0 warnings
test -d ../games/<FIRST_CONSUMER> || echo "../games missing: stop and report"
for g in <DOWNSTREAM_CONSUMERS>; do
  cargo check --manifest-path ../games/$g/Cargo.toml
  cargo check --manifest-path ../games/$g/Cargo.toml --features editor
done
scripts/check_wasm.sh    # REQUIRED if `git diff --cached --stat` touches any file under
                         # crates/engine_core, crates/renderer, crates/audio, crates/input, crates/common
```
Add `cargo test` per consumer when the batch section says the batch changes
behaviour they exercise.

## Report shape

One message: the exact summary lines of each gate, and the list of crate
roots your staged diff touches (so the wasm-gate decision can be audited);
every batch item with "done", "done differently because …", or "not done
because …" (with the grep or the compile error); the one non-obvious decision
you made, if any; any production hunk outside the batch's scope, called out
by file and line; then, verbatim, the output of
`git status --porcelain -- <the paths you touched>` (must show only `A`/`M`/`D`
in the first column — nothing `??`, nothing modified-but-unstaged) and the
tail of `git diff --cached --stat`.
