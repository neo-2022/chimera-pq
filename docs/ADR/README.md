# ADR Process (Mesh/MVP)

## Rule
Any non-trivial decision affecting runtime behavior, CLI contracts, reliability,
security, or architecture must be documented in an ADR on the same day and in
the same implementation pass.

A change is not considered done until:
1. code/tests are updated;
2. ADR entry is added/updated;
3. evidence commands are listed.

## Naming
Use one of:
- `000N-*.md` for broad architecture records;
- `ADR_<TOPIC>_<YYYY-MM-DD>.md` for focused implementation decisions.

## Minimal ADR Template
- Context
- Decision
- Evidence (files + commands)
- Consequences
- Next step

## Current ADRs
- `0001-mvp-scope.md`
- `0002-full-cef-track-gates.md`
- `ADR_MESH_CLI_OPTIONS_PARSE_CONTRACT_2026-05-21.md`
