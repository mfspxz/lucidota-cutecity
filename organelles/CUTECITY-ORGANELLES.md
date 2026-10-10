# CuteCity as Organelle Set (city-025)

**Status:** manifests written, staged. **Dependency OPEN:** `org-002`
(parkwreck-organelle-host rebuild) is another domain's item — these organelles
are specified against the organelle contract and behavior-tested against the
Rust crates, but not yet hosted. Nothing here claims to run.

## Contract (per SHAMBHALA organelle manifest)

Each organelle: 2-char codon `id()`, `handle(ctx, input) → ToolOutput`,
`mutation_class` (ReadOnly | Write | ExternalEffect), `declared_valence`
(Proven | Candidate | Tainted), `requires_capability`, `input_schema`
(JSON Schema).

## Organelles

### `cm` — CMS Content (cute-cms)

- **Mutation:** Write (admin mutations) / Read (public reads)
- **Valence:** Candidate — content is unverified until voice-passed
- **Capability:** `operator` for mutations; none for reads
- **Implements:** `cute-cms` crate (model, validate, slug, schema, admin, media)
- **Inputs:**
  - `{op: "validate_post", post}` → `{ok, errors[]}`
  - `{op: "slugify", title}` → `{slug}`
  - `{op: "voice_pass_ok", voice_passed, passed_by}` → `{ok}`
  - `{op: "presign", method, bucket, key, expires_secs}` → `{url}` (key material via capability, never in input)
  - `{op: "admin_list", resource}` → `{rows}` (requires operator capability; secret-gated upstream)
- **Behavioral equivalence:** `cargo test -p cute-cms` (33 tests) is the
  reference behavior. The host must reproduce: validation accept/reject,
  slugify outputs, presign determinism, gate fail-closed.

### `fd` — Feed Publisher (cute-feed + cute-pub)

- **Mutation:** ReadOnly (pure generation from snapshot)
- **Valence:** Proven — deterministic; byte-identical output for same input
- **Capability:** none
- **Implements:** `cute-feed` crate + `cute-pub` binary
- **Inputs:**
  - `{op: "json_feed", meta, items[], page}` → `{feed.json}`
  - `{op: "rss_feed", meta, items[], page}` → `{feed.xml}`
  - `{op: "public_api", snapshot}` → `{v1/posts.json, …}`
  - `{op: "sitemap", snapshot}` → `{sitemap.xml}`
- **Behavioral equivalence:** `cargo test -p cute-feed` (6 tests) + the
  staged artifacts in `~/workspace/cutecity/staging/` are the golden outputs.
  The host must reproduce them byte-for-byte from
  `d1-snapshot-2026-10-07.json`.

### `vr` — Route Verifier (cute-routes)

- **Mutation:** ReadOnly (probes)
- **Valence:** Proven
- **Capability:** none
- **Implements:** `cute-routes` binary
- **Inputs:** `{op: "verify", base}`, `{op: "accept_matrix", base, path}`, `{op: "timing", base}`
- **Golden:** `route-verify-2026-10-07.json` (29/29 green).

### `bk` — Backup (cute-backup)

- **Mutation:** Write (writes scratch DB only, never prod)
- **Valence:** Candidate (restore verified, live export post-freeze)
- **Capability:** `operator` for live-export
- **Inputs:** `{op: "dump_schema", snapshot}`, `{op: "restore_check", snapshot}`

### `gd` — Guard Tour (cute-guard)

- **Mutation:** Write (check-ins, incidents, alerts append sealed receipts)
- **Valence:** Candidate — full organelle interface implemented
  (`cute-guard/src/organelle.rs`, `cargo test -p cute-guard` 18/18);
  host verification pending
- **Capability:** `guard-firm`
- **Implements:** `cute-guard` crate (model, receipt, engine, organelle);
  SPEC.md next to the crate; manifest `src/manifests/gd.json` embedded
- **Inputs:**
  - `{op: "register_site", site, checkpoints[]}` → `{site_id}`
  - `{op: "define_tour", tour}` → `{tour_id}`
  - `{op: "check_in", checkin, scanned_tag?, sealed_ts}` → `{receipt, outcome}`
    (tag/geofence verified; rejections sealed as established observations)
  - `{op: "report_incident", incident, sealed_ts}` → `{receipt, incident_id}`
    (opens as `claimed`; supervisor authorizes later)
  - `{op: "clock_in", guard_id, site_id, ts, sealed_ts, location?}` → `{receipt}`
  - `{op: "clock_out", guard_id, ts, sealed_ts}` → `{receipt}`
  - `{op: "panic", guard_id, site_id, ts, sealed_ts}` → `{receipt, alert}`
  - `{op: "tour_status", tour_id, now_ts}` → `{stops: [{checkpoint_id, state}]}`
  - `{op: "missed_checkpoints", tour_id, guard_id, now_ts}` → `{alerts[]}`
- **Behavioral equivalence:** `cargo test -p cute-guard` is the reference
  behavior, including the full patrol cycle (clock in → check in → tour
  status → clock out) producing one verifiable hash chain. Envelopes are
  UNSIGNED in v0.1 (signing waits on the authority-key custody decision) —
  the host must not treat the seal as an auth signature.
- **Odin feed:** sealed check-in/alert receipts are the Odin Vancouver map
  layer (guard positions, checkpoint pins, alert pins). Receipts only —
  the map never gets raw location streams.

## Mesh organelles (pouw-mesh)

The PT proof-of-useful-work mesh as organelles. Contract: the unified
organelle trait (`pouw-mesh/rs/src/organelle.rs` — merges the go50/krx
`name/version/call/authority` shape with the CuteCity
codon/mutation-class/valence/capability/input-schema shape). Manifests in
`pouw-mesh/rs/src/manifests/*.json`, embedded at compile time.
Reference behavior: `cargo test` in `pouw-mesh/rs` (28/28).

### `sl` — Settlement Ladder

- **Mutation:** Write (advances claim rungs) · **Valence:** Candidate · **Capability:** none
- **Inputs:** `{op: "new", claim_id}`, `{op: "climb", claim_id, evidence_ok, endorsed}`,
  `{op: "status", claim_id}` → `{rung}`. Never skips rungs.

### `mr` — Merkle Roots

- **Mutation:** ReadOnly · **Valence:** Candidate · **Capability:** none
- **Inputs:** `{op: "root", leaves[]}` → `{root_hex}`, `{op: "prove", leaves[], index}` →
  `{proof[]}`, `{op: "verify", leaf_hex, proof[], root_hex}` → `{ok}`.

### `oc` — Oracle

- **Mutation:** ReadOnly · **Valence:** Candidate · **Capability:** none
- **Inputs:** `{op: "judge", tier: "local"|"mutual", claim, ...}` → `{verdict}`.
  Tier-3 juries live in `jy`; `oc` refuses `mesh` tier (fail-closed routing).

### `pc` — PT Coin

- **Mutation:** Write · **Valence:** Candidate · **Capability:** `coin-minter`
- **Inputs:** `{op: "mint", ...}`, `{op: "burn", ...}`, `{op: "balance", actor_id}`,
  `{op: "audit"}` → `{conservation_ok}`. Mint economics pluggable (`Economics::rate_for`);
  curve parameters are Zack's call.

### `jy` — Jury

- **Mutation:** Write (ballot commits) · **Valence:** Candidate · **Capability:** none
- **Inputs:** `{op: "verify_selection", ...}` → `{ok}`, `{op: "commit_ballot", ...}` →
  `{commit_hex}`, `{op: "tally", votes[]}` → `{useful, total, quorum}`,
  `{op: "slashable", voted_useful, evidence_verifies}` → `{slashable}`.
  Selection is private (needs secret keys); verification is public.

## Open

- org-002 host rebuild (other domain) — these organelles plug in when ready.
- Live D1 writes need the freeze to lift + admin key (Zackary).
