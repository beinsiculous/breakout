---
name: sprint-planning
description: Group open Studio Board issues into sprints - batches that can be developed and deployed together - and record each as a repo milestone plus the board's Sprint field. Use when asked to plan a sprint, batch the backlog, group issues, decide what to work on next, or when the board has accumulated ungrouped issues.
---

# Sprint planning

A **sprint** is a batch of issues that can be developed *and deployed* together. It is not
a timebox — nothing here has a due date. The test a batch must pass:

1. **One shippable outcome.** Finishing the batch produces something that can go out.
2. **Dependency-closed.** Everything the batch needs is either inside it or already done.
   Anything outside it is named as a *gate*.
3. **Worth naming.** If you cannot write its one-line theme, it is not a sprint — it is a
   pile.

Sprints routinely **span repos** (an engine change plus the game that consumes it). GitHub
milestones cannot, which is why every sprint is recorded twice — see step 4.

## 1. Gather — one call

```sh
gh project item-list 1 --owner beinsiculous --limit 200 --format json
```

Every row carries what you need, so no per-repo join is required:

| field | what it gives you |
|---|---|
| `id` | the `PVTI_…` board item id — what `gh project item-edit` needs |
| `content.repository` / `.number` / `.url` / `.title` / `.body` | the issue itself |
| `labels` | `tech-debt`, `bug`, `enhancement`, … |
| `milestone` | current sprint, with **title and description** |
| `priority` / `phase` / `status` | the board's own fields |

Filter to `status != Done`. Read the **bodies** — acceptance criteria and stated
dependencies live there, and they are what determine the gate chain.

```sh
gh project field-list 1 --owner beinsiculous     # field ids, and the Sprint options
```

## 2. Cluster

Group by shippable outcome, not by label or by crate. Signals that two issues belong in
the same batch:

- they touch the same surface and would collide if worked separately;
- one's acceptance criteria mention the other;
- they share a deploy target (the same site build, the same game binary, the same store
  submission);
- shipping one without the other leaves a visibly half-done feature.

Signals they do **not**: same `Phase` but independent; same crate but unrelated; both
tagged `tech-debt`.

Leave out on purpose:
- **Low-priority backlog issues** — worked opportunistically, never scheduled.
- Anything blocked on an external decision that has not been made.

## 3. Order inside the batch, and name the gates

State the sequence explicitly, by issue number, and say what gates what. A gate is any
issue that must land before another can start — including one in a *different* repo, or
an external event (an asset delivery, a store review, a decision from Jesse).

## 4. Record it twice — on purpose

GitHub gives no cross-repo milestone, so a sprint that spans repos cannot live in one
place. This is a deliberate duplication of the kind the working set's DRY rule allows —
"where a rule genuinely must exist twice, a test or a parity check holds them together" —
and `scripts/check-sprint-sync.sh` is that check. Never write one without the other.

### 4a. The milestone (per repo)

Title = the sprint name. Description follows this shape, which is what
`insiculous_2d/.claude/commands/continue.md` reads to pick the next task:

```
<one line: what this batch is and what shipping it means>
ORDER: <gate chain by issue number>.
<external gates, if any>
<what is deliberately excluded, and why>
```

A real one, from `Deion Pipeline (E+F)`:

> Asset pipeline remainder + Deion asset production. ORDER: F3 gen_tiles (#69) gates E5
> (#11); E7 (#10), E8 (#67), F2 sync (#68) independent; F4 placeholders (#70) then F5
> first-animated-Deion (#71) as capstone. Gates the Re-skins sprint.

```sh
gh api repos/beinsiculous/<repo>/milestones -f title="..." -f description="..."
gh issue edit <N> -R beinsiculous/<repo> --milestone "<title>"
```

Create the milestone in **every** repo the sprint touches, with the same title and the
same description. Cross-repo issues then read identically from either side.

### 4b. The `Sprint` field (spans repos)

Set `Sprint` on every issue in the batch, in whichever repo it lives. This is the only
record that holds a cross-repo batch together.

**Batch the writes into one request.** `gh project item-edit` sends one HTTP request per
issue, and GitHub's *secondary* rate limit (~500 content-generating requests/hour, separate
from the documented 5,000/hr quota and not raised by any paid plan) will stop a sprint-sized
backfill partway. Alias the mutations instead — any number of field writes in a single call:

```sh
gh api graphql -f query='
mutation {
  a0: updateProjectV2ItemFieldValue(input: {projectId: "PVT_…", itemId: "PVTI_…a",
      fieldId: "PVTSSF_…", value: {singleSelectOptionId: "…"}}) { projectV2Item { id } }
  a1: updateProjectV2ItemFieldValue(input: {projectId: "PVT_…", itemId: "PVTI_…b",
      fieldId: "PVTSSF_…", value: {singleSelectOptionId: "…"}}) { projectV2Item { id } }
}'
```

Build the aliased mutation from the `item-list` output, and read current values first so a
re-run only writes what differs — that keeps a resumed run cheap as well as correct. Use
the single-item form below only for a one-off:

```sh
gh project item-edit --project-id <PVT_…> --id <PVTI_…> \
  --field-id <sprint-field-id> --single-select-option-id <option-id>
```

Adding a new sprint means adding its option to the field first. Do it idempotently —
check `gh project field-list` and skip if the option is already there. Note that **renaming
an existing option invalidates every value set from it**, silently clearing those issues;
re-run the backfill after any rename.

Milestone work is REST (`gh api repos/{owner}/{repo}/milestones`) and unaffected by the
GraphQL secondary limit — if the board is throttled, the milestone half can still proceed.

## 5. Verify

```sh
scripts/check-sprint-sync.sh      # from the working-set root
```

It reports any issue whose `Sprint` and milestone disagree, any milestone with no matching
`Sprint` option or the reverse, and any milestoned issue with no `Sprint` set. Exit 0 or
the plan is not finished.

## 6. Report

Name each sprint, its issue count, its gate chain, and — explicitly — every open issue you
left **out** of every sprint, with the reason. Unassigned work that nobody mentions is how
a backlog rots.

## Re-planning an existing board

Do not silently re-cut sprints that are partly done. Propose changes and adjudicate them
with the user: moving an issue out of a started sprint changes what someone is already
working toward.
