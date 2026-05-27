# Routing

Default runtime mode:

- `split` (required default).
- CHIMERA observes system/application flows and preserves direct path for
  resources that are reachable directly.
- CHIMERA switches only unreachable resources to gateway/proxy path.
- `full` mode is explicit operator override and is not default.

MVP policy precedence:

1. exact domain;
2. domain suffix;
3. CIDR;
4. protocol + port;
5. default route.

Every decision must produce an explanation string.

## Split Auto-Failover Contract

Required behavior in split mode:

1. Default decision is direct unless policy says otherwise.
2. If direct path to a resource fails and CHIMERA path succeeds, decision for
   this resource flips to CHIMERA path.
3. Flip is per-resource (domain/IP binding), not global.
4. Other resources remain on direct path.
5. Decision is persisted in adaptive state with timestamp.
6. Periodic recheck may return resource back to direct when direct path is
   healthy again (with anti-flap hysteresis/TTL).

Current implementation status:

- `partial`:
  - split-mode routing and adaptive per-domain decisions are implemented;
  - `site-auto-resolve` and `site-auto-watch` update per-domain route choices;
  - `site-auto-bootstrap` seeds adaptive checks from known domains.
- `not yet complete`:
  - universal passive per-flow detection for every app/resource in runtime
    datapath is not fully closed yet.
