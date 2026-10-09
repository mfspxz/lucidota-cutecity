# Staging Environment (city-021)

## What "staging" means here

`~/workspace/cutecity/staging/` is the content staging area: every artifact
that would go public lives here first, hashed into the voice-pass pipeline
(`voice-pass.py`), and only reaches prod after Zackary's approval.

## Parity with prod

Parity is verified by regeneration, not by a standing server:

```
# regenerate from the frozen snapshot and diff against what's staged
cute-pub all --out /tmp/staging-parity < d1-snapshot-2026-10-07.json
diff -r /tmp/staging-parity ~/workspace/cutecity/staging
```

A clean diff = staging matches the snapshot. A dirty diff = re-stage and
re-voice-pass the changed area (the pipeline's hash check enforces this).

## Live staging URL — parent's call

Options (not chosen here — DNS/network is CITY-1's lane):
1. **workers.dev route** (e.g. `cutecity-staging.<account>.workers.dev`) —
   no zone DNS needed; I can provision this via the v4 API when told to.
2. **`staging.cutecity.cc`** — needs CITY-1 (custom domain attach, same
   pattern as tricycles.cutecity.cc, seq 479).

Either way: the staging worker serves ONLY staged artifacts, secret-gated,
no public routes — same privacy posture as prod.

## Current staged areas

| Area | Path | Voice pass |
|------|------|------------|
| feed | staging/feed/ | ⏳ pending |
| og | staging/og/ | ⏳ pending |
| portfolio | staging/portfolio/ | not yet staged |
| blog | staging/blog/ | not yet staged |
| directory | staging/directory/ | not yet staged |
| districts | staging/districts/ | not yet staged |
| saltspringslasher | staging/saltspringslasher/ | not yet staged |
| tricycles | staging/tricycles/ | not yet staged |
| odin-map | staging/odin-map/ | not yet staged |
| copy | — | not yet staged |
