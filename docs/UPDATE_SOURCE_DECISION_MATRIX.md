# Update Source Decision Matrix

Status: active

This document defines the runtime update decision contract for installed
CHIMERA nodes. It is intentionally strict: lower-trust mirrors may help with
delivery outages, but they must not silently outrun a higher-trust source that
has already proved a release tuple.

## Trust Order

1. GitHub Release/Latest
2. Gitvers public mirror
3. Trusted peer update source

For `chimera-sh -connect <peer>`, the peer fallback is narrowed to the selected
peer's `update_bootstrap_url`. The general peer list is not a silent substitute
for a different selected peer.

## Verified Release Tuple

A source becomes authoritative only after CHIMERA has a fully verified tuple:

- `version`
- `archive_sha256`

Metadata alone is not enough. A source that exposed a version string but did
not yield a verified checksum is still an outage candidate, not a version
authority.

## Decision Rules

1. If a higher-trust source is unreachable before a verified tuple exists,
   CHIMERA may continue to the next source.
2. If a higher-trust source is invalid or inconsistent, CHIMERA fails closed
   and does not try a lower-trust substitute.
3. If a higher-trust source yields a verified newer tuple, CHIMERA installs
   that release from that source.
4. If a higher-trust source yields a verified current or stale tuple, CHIMERA
   stops the search, emits `chimera_update=no_newer_release`, and does not let
   a lower-trust source outrun it in the same round.
5. If a higher-trust source yielded a verified newer tuple but the archive
   delivery/install path was unavailable, CHIMERA may try a lower-trust mirror
   only for the exact same `{version, sha256}` tuple.
6. If a lower-trust candidate differs from the verified authority version,
   CHIMERA blocks with `trusted_version_divergence`.
7. If a lower-trust candidate matches the verified authority version but
   differs by checksum, CHIMERA blocks with `trusted_checksum_divergence`.
8. CHIMERA never downgrades to an older semver release.
9. `same version + different checksum` and `same version + missing local
   checksum` are fail-closed conditions.

## Matrix

| Highest source state seen so far | Lower source state | Action |
| --- | --- | --- |
| No verified source yet; current source unreachable | Next source newer/current/stale and internally valid | Continue to next source and evaluate it normally |
| Higher source invalid metadata/version/checksum | Any lower source | Block |
| Higher source verified newer tuple; install path works | Any lower source | Install from higher source; stop |
| Higher source verified newer tuple; install path unavailable | Lower source exposes same version and same sha | Allow mirror fallback install |
| Higher source verified newer tuple; install path unavailable | Lower source exposes different version | Block with `trusted_version_divergence` |
| Higher source verified newer tuple; install path unavailable | Lower source exposes same version but different sha | Block with `trusted_checksum_divergence` |
| Higher source verified current tuple | Any lower source | Emit `no_newer_release`; stop |
| Higher source verified stale tuple | Any lower source | Emit `no_newer_release`; stop |
| GitHub unavailable; Gitvers verified current/stale | Peer newer | Emit `no_newer_release`; stop at Gitvers |
| GitHub and Gitvers unavailable | Peer verified newer | Install from peer |
| All trusted sources unreachable | n/a | Emit `chimera_update=unavailable` and keep installed release |

## Security Boundary

Checksum verification proves bundle integrity for that source. It does not by
itself prove full provenance equivalence across GitHub, Gitvers, and peer
mirrors. Until signed release manifests exist, Gitvers and peer fallbacks
remain operator-trusted distribution paths, not independent authorities.
