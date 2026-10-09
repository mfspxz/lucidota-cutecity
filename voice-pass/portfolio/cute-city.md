# Cute City — the website & world

**Status:** BETA · **Visibility:** public · **Slug:** `cute-city`

> The site where all software launches. Interactive fiction front,
> dispatch feed, release windows, and the city map.

## What was built
- Vinext SSR worker on Cloudflare, D1 content store, R2 private media.
- Dispatch feed (RSS + JSON), public read API, release windows per platform.
- Districts: first-address, release-desk, harbor, krampus (open);
  library, void (rumour); clowncil, parkwreck (mist).

## Operator role
Built and operated by Zackary Camoran. Content voice-passed before publish.

## Results
- Live at https://cutecity.cc (apex). Health ok, admin secret-gated.
- 3 dispatches published (Aug 2026).

## Limitations
- Worker source unrecoverable (compiled bundle only) — rebuild planned.
- Feed generation nondeterministic (observed 2026-10-07).

## Evidence
- Route verification: 29/29 green (2026-10-07).
- Ship report: `~/workspace/cutecity/SHIP-REPORT-2026-10-07.md`.

---
*STAGED — voice pass pending. Do not publish.*
