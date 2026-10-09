# Voice-Pass Content Pipeline (city-013)

**Rule: no public copy ships without Zackary's voice pass. This pipeline is the
only staging → prod path, and it fails closed.**

## States

```
DRAFT → STAGED → VOICE-PASSED → PUBLISHED
              ↘ REJECTED (back to DRAFT with notes)
```

## How it works

1. **DRAFT**: content authored in `~/workspace/cutecity/staging/<area>/`.
2. **STAGED**: `voice-pass.py stage <area>` copies artifacts to
   `~/workspace/cutecity/voice-pass/<area>/` and writes a manifest
   (`manifest.json`) with sha256 of every file + `voice_passed: false`.
3. **VOICE-PASSED**: Zackary reviews and approves. Approval is recorded by
   `voice-pass.py approve <area> --by "Zackary Camoran" --note "..."`,
   which flips `voice_passed: true` and records actor + timestamp.
   (Only Zackary — or an explicit delegation he names — may approve.)
4. **PUBLISHED**: `voice-pass.py publish <area>` verifies the manifest
   (hashes match, `voice_passed: true`, approver non-empty) and only then
   allows the deploy step. Any mismatch → REFUSED, ledger entry.

## Manifest (`manifest.json`)

```json
{
  "area": "feed",
  "staged_at": "2026-10-07T05:00:00Z",
  "files": {"feed.json": "sha256:…", "feed.xml": "sha256:…"},
  "voice_passed": false,
  "passed_by": null,
  "passed_at": null,
  "note": null
}
```

## Areas

- `feed` — feed.json / feed.xml / v1/*.json / sitemap.xml
- `og` — social card snippets
- `saltspringslasher` — subdomain content scaffold
- `tricycles` — subdomain content scaffold
- `districts` — new district scaffolds
- `odin-map` — Odin C2 map frontend
- `copy` — any other public copy

## Enforcement

- `cute-cms::validate::voice_pass_ok()` is the code-level gate (tested).
- `publish` re-hashes every file; a single changed byte without re-approval
  fails the publish.
- Every stage/approve/publish/reject writes a ledger entry (actor-attributed).
