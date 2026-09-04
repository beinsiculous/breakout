---
name: handoff-loop
description: The three-model delivery loop - the interactive session plans, two other vendors' reviewers attack the plan until it is settled, a third-party CLI (gemini) executes one batch from a handoff prompt tied to the plan file, and the planner plus the counterpart reviewer code-review the result before the planner commits it. Use when a body of work is big enough to batch, when the user wants to spend the planning model on judgment rather than typing, or when they invoke /handoff-loop. Builds on adversarial-review; does not replace it.
---

# Handoff loop (plan → settle → delegate → review → commit)

The **planner** (you, the interactive session) owns the plan, the gates, the
adjudication, the accepted fixes and every commit. The **executor** (gemini,
driven by the user in another window) writes the code for one batch at a time
from a handoff prompt. The **reviewers** are the planner and the counterpart
CLI (`kimi` when the planner is Claude, `claude` when it is Kimi), through the
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
2. **All three models are in on every plan: the planner drafts, the other two
   review.** The planner writes the first draft (the user's ruling, Sep 3
   2026: the draft is one voice, the feedback is two), then the counterpart
   reviewer AND the executor's CLI each review it in plan mode, each on its
   own file (`review-N.md` and `review-N-<reviewer>.md`), dispatched together
   on the same snapshot. The executor reviews because it will build the
   thing — it sees the shapes it would have to type, and its objections are
   cheaper here than in a report marked INCOMPLETE. Adjudicate each review
   on its own, write one rebuttal covering both, revise, repeat until the
   user calls it settled. Record decisions in the plan itself; the rebuttals
   explain them. The same three-model rule applies to every batch-section
   correction in step 4: a correction over the hook's threshold goes to both
   reviewers in code mode before it commits.
3. Every correction that lands later — a dead-API list that grew, a ruling
   that changed during a review — goes **into the batch that will act on it**,
   never into a side section. The executor reads one section; anything filed
   elsewhere is missed (the batch-2 lesson).
4. **Before each handoff, re-verify the batch section against the tree.**
   Earlier batches move and delete things: grep every `file:line`, symbol and
   count the section names, note what a prior batch already did, name the
   files near the size ceiling, and commit those corrections into the section
   (its own pathspec commit) before writing the handoff. Batch 3's section
   still named a file batch 2 had deleted and a call-site count that was
   four off; the executor would have chased both.

## 2. Hand off one batch

Write `review/<subject>/handoff-<batch>.md` from `prompts/handoff-batch.md`
(the template) and give the user its path. It must carry, verbatim or by exact
section reference: the plan path and section, the design sections with the
target shapes (and which of their parts earlier batches already shipped), the
ground rules (branch, "stage everything you touched including new files, do
not commit", "report INCOMPLETE rather than stop silently"), the gates with
their mechanical conditions, the standing rules from earlier batches (show the
grep for every deletion; before deleting the only test of an API, check the
consumers; a gate an earlier batch introduced — a tag gate, a games script —
applies to every later one), and the report shape you want back — which must
end with the `git status --porcelain` of the batch scope and the
`git diff --cached --stat` tail, so a finished batch is distinguishable from
an abandoned one.

While a batch is out — including any second pass — the planner **neither edits
nor runs cargo** in that checkout. Verification is the executor's job until
the report arrives; two cargo runs in one target directory block each other or
produce spurious red gates, and an edit of yours lands in the executor's diff.
Doc-only commits elsewhere in the tree are tolerable if pathspec-scoped, but
prefer to wait.

## 3. Take the result back

1. **Reconcile before anything else.** Compare the report's `--stat` tail with
   the actual `git diff --cached --stat`, and run `git status --porcelain --
   <batch scope>`: nothing untracked (`??`) and nothing modified-but-unstaged
   (` M`). A mismatch means an abandoned or half-staged batch: do not review
   it; ask the user, and recover with `git stash push -u -- <scope>` (never
   `git checkout -- .`, which destroys the executor's work).
2. **Run the gates yourself** — never take a green report on trust — the
   repo's test and lint commands, the downstream consumers' checks (verify
   `../games` resolves to the expected working set first), the wasm gate
   whenever the staged diff touches a file under the crate roots it covers,
   and every standing gate earlier batches added.
3. **Snapshot the exact bytes the reviewers will read:**
   `git diff --cached > review/<subject>/draft-<batch>.diff`. Send it to the
   counterpart reviewer. On a diff over a few thousand lines the reviewer
   outruns the tool timeout, so run it detached with its failure signals
   captured — `nohup scripts/request-review.sh code <draft> --reviewer=<r>
   > <log> 2>&1 &`, record `$!` and the dispatch timestamp, wait for that PID
   to exit (poll it; the harness kills a foreground wait at ten minutes), then
   check: exit code 0, the log free of `INCOMPLETE`/error lines, and the
   review file ending in a Verdict section. Anything else is **no review**;
   re-run or split the diff.
4. **Write your own review** to `review/<subject>/review-N-<you>.md` while the
   counterpart runs, where N is the number the script assigned its file
   (numbering is per subject and continues across batches; `rebuttal-N.md`
   pairs with `review-N.md`, never reused): check every item of the batch
   landed (grep the staged tree for each symbol it names), check stale
   references in the guides — every crate guide, README and doc that names a
   deleted or renamed thing, not only the lines the batch listed — and read
   every non-test hunk.
5. **Adjudicate both** with the user (`adversarial-review` § Code mode) and
   write `rebuttal-N.md`. Verify each claim against the tree before accepting
   it; a finding that asks for a gate you already ran is rebutted with the
   log line.
6. **Apply the accepted fixes yourself.** The batch is back, so the no-edit
   rule has lifted, and you hold the whole context: a fixes prompt to the
   executor costs more than the fix (Jesse's ruling, Sep 3 2026). Fix in the
   checkout, stage, re-run every gate from step 2, and list in the rebuttal
   every file your fix hunks touch and what each does. A fix may only do what
   a finding names — a pin, a doc line, a deletion, a rename, a signature the
   finding specifies; grep-confirmation is only sound for that narrow kind.
   Two things go back through step 3 as their own diff instead: a fix that
   changes behaviour or grows past a few dozen lines, and a batch that turns
   out half done (batch 2's missed list) — that one goes back to the executor
   as `review/<subject>/<batch>-fixes-for-<executor>.md`, numbered, one file
   and one contract per item, gates at the end.
7. **Assert what the commit contains.** Snapshot the fixed tree
   (`git diff --cached > review/<subject>/draft-<batch>-fixed.diff`) and diff
   it against the reviewed snapshot: the delta must touch exactly the files
   the rebuttal listed in step 6 and nothing else — that is the honest
   statement of what door 1 covers, "the reviewed bytes plus the enumerated
   fix hunks". Then `git diff -- <batch scope>` must be empty (index ==
   working tree; a pathspec commit takes the working tree, which this repo's
   hook documents) and nothing outside the scope may be staged.
8. **Commit** with the review asserted, message and pathspec both from files:
   ```sh
   ADV_REVIEWED=1 git -C <repo> commit -F <message-file> --pathspec-from-file=<scope-file>
   ```
   The commit hook reads any `(` it cannot attribute to a quoted string as a
   subshell it cannot follow and denies the command: a `$(…)` pathspec is one,
   and a multi-line `-m` message defeats its quote stripping, so parentheses
   inside the message read the same way. It also denies any shell command
   whose text merely contains a commit invocation, so files that quote one
   are written with the Write tool, not a heredoc. `-F` and
   `--pathspec-from-file` avoid all of it. The message names the executor as
   author of the change, both reviewers with their files, your own fix hunks
   as not re-reviewed, and the harness attribution lines. Then mark the batch
   done in the plan **with its own pathspec** (`-- coordination/<effort>/plan.md`),
   never a bare commit that sweeps whatever else is staged; anything staged
   outside the batch scope gets its own commit too.

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
  that reconstruct production logic, both have called live API dead, and one
  has asked for a gate that had already run. Verify the claim against the
  tree before accepting it.
- One batch out at a time. Sequential is the simplicity that makes the
  review protocol sound; parallel batches were rejected in this repo's plan
  review for that reason.
- A second review round is not the default. It exists for fixes that change
  behaviour; log lines, doc lines, renames and a struct reshape are gated,
  enumerated in step 7, and not re-reviewed.
- Everything unfinished becomes an issue before the effort reports done
  (`file-issue`).
