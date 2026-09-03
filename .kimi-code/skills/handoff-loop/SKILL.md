---
name: handoff-loop
description: The three-model delivery loop - the interactive session plans, two other vendors' reviewers attack the plan until it is settled, a third-party CLI (gemini) executes one batch from a handoff prompt tied to the plan file, and the planner plus the counterpart reviewer code-review the result before the planner commits it. Use when a body of work is big enough to batch, when the user wants to spend the planning model on judgment rather than typing, or when they invoke /handoff-loop. Builds on adversarial-review; does not replace it.
---

# Handoff loop (plan → settle → delegate → review → commit)

The **planner** (you, the interactive session) owns the plan, the gates, the
adjudication and every commit. The **executor** (gemini, driven by the user
in another window) writes the code for one batch at a time from a handoff
prompt. The **reviewers** are the planner and the counterpart CLI
(`kimi` when the planner is Claude, `claude` when it is Kimi), through the
`adversarial-review` skill's code mode. The user adjudicates every finding
and decides when a plan or a batch is settled; nothing here removes them
from a judgment point.

Why it works: the expensive model spends its tokens on the decisions that
compound (the plan, what a finding means, what to keep), the executor spends
its tokens on the typing, and no model reviews its own work.

## 1. Plan

1. Draft in your harness's plan mode with the user (`adversarial-review` §
   Plan mode). The plan lives at a **tracked** path, not in `review/`:
   `coordination/<effort>/plan.md` (this repo) or the equivalent the repo's
   guide names — `review/` is transient and does not survive a fresh clone.
   The batches are the unit of work: each names its files, its target shapes,
   which gates apply (mechanically — "if the diff touches a file under these
   crate roots, run X"), and what it deliberately leaves out.
2. Review it with **both** reviewers, each on its own file, adjudicate each
   on its own, write the rebuttal, revise, repeat until the user calls it
   settled. Record decisions in the plan itself; the rebuttals explain them.
3. Every correction that lands later — a dead-API list that grew, a ruling
   that changed during a review — goes **into the batch that will act on it**,
   never into a side section. The executor reads one section; anything filed
   elsewhere is missed (the batch-2 lesson).

## 2. Hand off one batch

Write `review/<subject>/handoff-<batch>.md` from `prompts/handoff-batch.md`
(the template) and give the user its path. It must carry, verbatim or by exact
section reference: the plan path and section, the ground rules (branch,
"stage everything you touched including new files, do not commit"), the gates
with their mechanical conditions, the standing rules from earlier batches
(show the grep for every deletion; before deleting the only test of an API,
check the consumers), and the report shape you want back — which must end
with the `git status --porcelain` of the batch scope and the
`git diff --cached --stat` tail, so a finished batch is distinguishable from
an abandoned one.

While a batch is out — including its fixes round — the planner **neither edits
nor runs cargo** in that checkout. Verification is the executor's job until
the report arrives; two cargo runs in one target directory block each other or
produce spurious red gates, and an edit of yours lands in the executor's diff.

## 3. Take the result back

1. **Reconcile before anything else.** Compare the report's `--stat` tail with
   the actual `git diff --cached --stat`, and run `git status --porcelain --
   <batch scope>`: nothing untracked (`??`) and nothing modified-but-unstaged
   (` M`). A mismatch means an abandoned or half-staged batch: do not review
   it; ask the user, and recover with `git stash push -u -- <scope>` (never
   `git checkout -- .`, which destroys the executor's work).
2. **Run the gates yourself** — never take a green report on trust — the
   repo's test and lint commands, the downstream consumers' checks (verify
   `../games` resolves to the expected working set first), and the wasm gate
   whenever the staged diff touches a file under the crate roots it covers.
3. **Snapshot the exact bytes the reviewers will read:**
   `git diff --cached > review/<subject>/draft-<batch>.diff`. Send it to the
   counterpart reviewer. On a diff over a few thousand lines the reviewer
   outruns the tool timeout, so run it detached with its failure signals
   captured — `nohup scripts/request-review.sh code <draft> --reviewer=<r>
   > <log> 2>&1 &`, record the PID, wait on it, then check: exit code 0, the
   log free of `INCOMPLETE`/error lines, and the review file ending in a
   Verdict section. Anything else is **no review**; re-run or split the diff.
4. **Write your own review** to `review/<subject>/review-N-<you>.md` while the
   counterpart runs, where N is the number the script assigned its file
   (numbering is per subject and continues across batches; `rebuttal-N.md`
   pairs with `review-N.md`, never reused): check every item of the batch
   landed (grep the staged tree for each symbol it names), check stale
   references in the guides, read every non-test hunk.
5. **Adjudicate both** with the user (`adversarial-review` § Code mode), write
   `rebuttal-N.md`, and turn the accepted findings into
   `review/<subject>/<batch>-fixes-for-<executor>.md`: numbered, one file and
   one contract per item, gates at the end. Fixes may only do what a finding
   names — a pin, a doc line, a deletion, a signature the finding specifies.
   A fix that changes behaviour beyond that goes back through step 3 as its
   own diff; grep-verification is only sound for the narrow kind.
6. **Re-verify** the way steps 1–2 did, confirm each fix item by grep, and
   then assert that the committed bytes are the reviewed bytes: `git diff --
   <batch scope>` must be empty (index == working tree; a pathspec commit
   takes the working tree, which this repo's hook documents) and nothing
   outside the scope may be staged.
7. **Commit** with the review asserted:
   `ADV_REVIEWED=1 git commit -F <message> -- <batch scope>`, naming the
   executor as author of the change and both reviewers with their files in
   the message, plus the harness attribution lines. Then mark the batch done
   in the plan **with its own pathspec** (`-- coordination/<effort>/plan.md`),
   never a bare commit that sweeps whatever else is staged.

## 4. Keep score

Maintain `coordination/<effort>/reviewer-comparison.md`: one row per review
(subject, reviewer, findings, real, false, policy rebuts, the notable catch
**quoted in the row** — the review files are transient — and wall time
measured as the review file's mtime minus the dispatch timestamp you
recorded). It is what makes "which reviewer should be the default" a number
instead of an impression, and the record the next effort reads.

## Rules that do not bend

- The planner never writes skip trailers and never commits an unreviewed
  diff over the hook's threshold; the executor never commits at all.
- A finding is adjudicated, not obeyed: both reviewers have asked for tests
  that reconstruct production logic, and both have called live API dead.
  Verify the claim against the tree before accepting it.
- One batch out at a time. Sequential is the simplicity that makes the
  review protocol sound; parallel batches were rejected in this repo's plan
  review for that reason.
- Everything unfinished becomes an issue before the effort reports done
  (`file-issue`).
