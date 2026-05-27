# Probe Access (Runtime)

`chimera probe access` checks site reachability and produces route recommendation
(`direct`, `gateway`, `unreachable`) for one or many targets.

## Single Target

```bash
chimera --lang en probe access \
  --url https://example.org \
  --timeout-sec 4 \
  --json
```

## Batch Targets

```bash
chimera --lang en probe access \
  --url https://example.org \
  --url-file configs/probe_targets.txt \
  --timeout-sec 4 \
  --json
```

`--url-file` format: one URL per line, empty lines and `# comments` are ignored.

## Auto-Apply Runtime Policy

```bash
chimera --lang en probe access \
  --url-file configs/probe_targets.txt \
  --apply-policy configs/policy.runtime.conf \
  --rule-id-prefix auto-probe \
  --json
```

Behavior:

- updates existing `exact:<domain>` rule if present;
- otherwise appends a new exact-domain rule;
- verifies resulting policy decision for that domain;
- writes policy atomically (temp file + rename).

Note: auto-policy is blocked for IP-literal targets and reported via
`target_error=policy_domain_ip_literal_not_supported`.

## Fail Gate for CI/Automation

```bash
chimera --lang en probe access \
  --url-file configs/probe_targets.txt \
  --fail-threshold 0 \
  --json
```

Exit code:

- `0`: `failed_total <= fail_threshold`
- `1`: `failed_total > fail_threshold`

JSON report contains `totals` and per-target rows in `targets`.

## Just Recipes

```bash
just probe-access-smoke-selfcheck
just probe-access-smoke
```

- `probe-access-smoke-selfcheck` validates recipe wiring and JSON shape.
- `probe-access-smoke` writes the latest snapshot to
  `docs/probe_access_latest.json`.
