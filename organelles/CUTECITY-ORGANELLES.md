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

## Open

- org-002 host rebuild (other domain) — these organelles plug in when ready.
- Live D1 writes need the freeze to lift + admin key (Zackary).
