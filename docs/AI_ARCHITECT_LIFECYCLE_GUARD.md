# AI Architect Lifecycle Guard

Status: active
Source: extracted from the user-supplied AI Architect Algorithm document on
2026-06-30 and made canonical for CHIMERA-PQ work.

## Purpose

This document makes the AI Architect lifecycle a permanent CHIMERA-PQ process
contract. It is not a product feature and it does not replace the MVP scope,
security rules, Truth-First rules, SSH-only runtime checks, or no-hardcode
rules from `AGENTS.md`.

The goal is simple: future sessions must leave a machine-checkable trail proving
that serious work went through the required lifecycle instead of being decided
by intuition or informal notes.

## Authority

The lifecycle is below the following authorities:

1. the user's latest explicit command;
2. `AGENTS.md`;
3. `CHIMERA-PQ_MVP_SPEC.md`;
4. `Agent.md`;
5. this document.

If this lifecycle conflicts with CHIMERA safety, secrecy, MVP-scope, no-local-
runtime, SSH-only, no-SOCKS, no-hardcode, or Truth-First rules, the CHIMERA rule
wins and the work must return to the earliest affected lifecycle stage.

## Mandatory Lifecycle

Non-trivial CHIMERA work must follow this order:

```text
START
 -> ANALYZE
 -> IMPACT_ANALYSIS
 -> RESEARCH_PLANNING
 -> RESEARCH
 -> ARCHITECTURE_SYNTHESIS
 -> IMPLEMENTATION
 -> VALIDATION
 -> POSTMORTEM
 -> DONE
```

The tactical cycle already recorded in `AGENTS.md`
(`ANALYSIS -> PLAN -> TEAM_CRITIQUE -> IMPLEMENTATION -> TEAM_CHECK -> FIX ->
RECHECK -> FINAL_AUDIT -> REPORT`) remains valid as a smaller work loop inside
the larger lifecycle. It does not replace the eight-stage lifecycle for
substantial work.

## Stage Contract

Each lifecycle stage must produce a stage report with:

- goal;
- input data;
- actions performed;
- constraints and invariants;
- assumptions and unknowns;
- risks and blockers;
- evidence references;
- outcome.

No stage is complete until it has:

- a stage report;
- council review;
- red-team review;
- council report;
- gate decision.

## Council Contract

For substantial CHIMERA work, the council must use real sub-agents when the
tool is available. The minimum roles are:

- architect;
- senior developer;
- tester;
- security engineer;
- DevOps/release engineer when release, install, runtime, remote stand, or
  operational checks are involved;
- critic-skeptic.

Council critique and proposals must explicitly use the interdisciplinary
research section from the source algorithm. The council is forbidden to limit
its critique to ordinary software-engineering opinion when the source algorithm
requires looking for transferable principles from other disciplines.

Council reports must record:

- role responsibilities;
- independent findings;
- interdisciplinary findings and knowledge-transfer checks;
- cross-review findings;
- red-team findings;
- accepted recommendations;
- rejected recommendations and reasons;
- remaining blockers or open questions.

The council does not replace the lead architect's responsibility. If a blocker
from security, testing, DevOps, or the critic remains unresolved, the final
status must be `not_done`, `partial`, `blocked`, `return`, or `stop`, not
`done`, `pass`, or `closed`.

## Gate Contract

Every stage ends with a gate decision:

- `pass`: the stage can move forward;
- `return`: the work must roll back to the earliest stage that can fix the root
  cause;
- `stop`: the work cannot continue until external conditions change.

Every gate must check:

- completeness;
- quality;
- risk;
- research sufficiency;
- red-team processing;
- readiness for the next stage.

`return` must name the rollback stage. Rolling back only to the latest stage is
forbidden when the root cause started earlier.

## Machine Artifact

Every substantial workline must maintain:

```text
docs/WORKFLOW_ATTESTATION.json
```

This JSON file is the machine-checkable proof that the lifecycle was followed.
It must not contain secrets, stand addresses, private credentials, unredacted
transit bytes, or machine-specific product defaults.

Every substantial workline must also preserve the exact extracted source
coverage catalog:

```text
docs/AI_ARCHITECT_ALGORITHM_COVERAGE.json
```

This catalog enumerates every extracted structural heading from the
user-supplied algorithm, including `PROMPT`, `STAGE`, `STEP`, `GATE`, role,
state, principle, report, transition and final `END` headings, with source line
numbers and a guard-checked digest. The current required count is 227
structural headings. A future session may add stricter fields, but it may not
delete, skip, reorder or reinterpret these required coverage items.

The heading catalog is not allowed to weaken the source algorithm. It is paired
with `normative_requirement_coverage` in `docs/WORKFLOW_ATTESTATION.json` for
mandatory line spans that contain binding instructions, especially the
interdisciplinary research, knowledge transfer, analogy evaluation and
mandatory other-discipline search sections. Those spans must have structured
proof, not only a heading reference.

The interdisciplinary source lists are binding, not examples that may be
collapsed into a generic flag. `docs/AI_ARCHITECT_ALGORITHM_COVERAGE.json` and
`docs/WORKFLOW_ATTESTATION.json` must preserve the exact extracted lists in
`interdisciplinary_source_lists`:

- `council_interdisciplinary_disciplines`, source lines 797-873, 37 items;
- `council_fundamental_knowledge_items`, source lines 883-927, 23 items;
- `stage_research_disciplines`, source lines 3161-3229, 33 items;
- `stage_research_fundamental_items`, source lines 3237-3295, 29 items;
- `knowledge_transfer_checks`, source lines 3307-3323, 8 items;
- `analogical_thinking_questions`, source lines 3415-3451, 19 items;
- `found_idea_evaluation_criteria`, source lines 3457-3481, 12 items.

For a `pass`/`done` attestation, `interdisciplinary_research` must include
`source_list_count=7` and `source_lists_checked` entries for all seven lists
above. A future session is forbidden to replace these lists with
`interdisciplinary_research=true`, a prose summary, or a software-only review.

The guard must also require full source text accounting. A `pass`/`done`
attestation must include `source_text_coverage` proving the original source
line count, non-empty line count, marker/bullet counts and contiguous
stage-to-line ranges from line 1 through line 5405. This prevents a future
session from covering only large headings while silently skipping bullets,
numbered checks or smaller instructions inside a section.

For a `pass`/`done` attestation, `docs/WORKFLOW_ATTESTATION.json` must include
`stage_reports.*.covered_required_item_ids` whose union exactly matches every
ID in `docs/AI_ARCHITECT_ALGORITHM_COVERAGE.json`.

For a `pass`/`done` attestation, every council individual report must also show
interdisciplinary findings, disciplines considered, fundamental principles,
knowledge-transfer checks, transfer risks, tradeoffs and rejected analogies.
The research stage report must include structured `transfer_principles` with
properties, invariants, constraints, transferable parts, non-transferable
parts, benefits, risks, tradeoffs and evidence. Generic strings are not enough
for the knowledge-transfer proof.

The council, research, architecture, implementation and postmortem reports must
use the concrete fields required by the source algorithm. Generic catch-all
fields such as `research_report`, `architecture_report`, `implementation_report`
or `postmortem_report` are not enough unless the concrete subfields are present:
researched theories/models/patterns, alternatives, rejection reasons, final
architecture, contracts, accepted decisions, changed components/interfaces/
contracts, technical debts, final assessment, confirmed/erroneous decisions,
lessons, open questions and updated research debt.

The guard for this artifact is:

```text
just workflow-attestation-guard
```

The locally safe process-only guard for a normal session is:

```text
just session-process-guard
```

For a handoff that must avoid local runtime/network checks, use:

```text
just handoff-process-check
```

`just handoff-check` and `just ship-readiness` include the workflow guard, but
they also run broader MVP/runtime/readiness checks. They are full gates, not
the safe process-only guard for a local session where CHIMERA runtime/network
actions are forbidden. The workflow guard is not included in `just mvp-check`,
because it is a process guard rather than a product MVP behavior check.

## Truth Boundary

The guard can verify structure, required fields, order, evidence presence,
rollback declarations, and forbidden claims. It cannot prove subjective quality,
perfect understanding, or that every possible risk in the universe was found.

Therefore all absolute claims must be expressed as evidence-backed claims:

- `verified_by:<file-or-command>` is allowed;
- `assumption` must stay marked as an assumption;
- `lab_pass` must not be promoted to `real_world_pass`;
- `prod_ready` is forbidden unless the real production release gate has the
  required evidence.

## Research Debt

Open research must be tracked in:

```text
docs/RESEARCH_DEBT.md
```

Research debt is allowed only when it is explicitly recorded with:

- what is not yet researched;
- why it is not blocking the current scope;
- what risk remains;
- what evidence would close it later.
