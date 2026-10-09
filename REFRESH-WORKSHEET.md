# D1 Content Refresh Worksheet (city-015)

**The D1 content is ~7 weeks stale (latest: 2026-08-18). Refreshing it is
Zackary's content call — this worksheet is the tooling; the words are his.**

## Current state (frozen snapshot 2026-10-07)

- **posts:** 3 published (the-doors-open-when-they-work, why-most-of-the-city-is-mist,
  how-city-hall-runs-the-website). Bodies/excerpts mostly absent from snapshot.
- **releases:** 6 platforms, ALL unpublished (PACKAGING × 2, PLANNED × 3, EXPLORING × 1).
- **portfolio_projects:** 3 public (cute-city, noter, cute-city-website).
- **district_overrides:** 8 (4 open, 2 rumour, 2 mist).

## Refresh flow

1. **Zackary writes** (markdown, anywhere — chat, file, paper):
   - New/updated dispatches (title, body, tags)
   - Release state changes (which platforms ship, versions, download URLs)
   - Project updates (results, limitations, evidence links)
   - District blurbs/names (the 8 overrides are blank)
2. **Builder stages**: content → `staging/copy/` → `voice-pass.py stage copy`
   → Zackary approves (`approve copy --by "Zackary Camoran"`)
3. **Publish**: `voice-pass.py publish copy` → CLEARED → admin writes to D1
   via the secret-gated API (needs admin key; post-freeze for D1 writes)
4. **Regenerate**: `cute-pub all` from the new snapshot → new staged
   feed/API/sitemap → voice pass → deploy.

## Prompts for Zackary (one at a time, per token discipline)

1. District blurbs: 8 districts need one-line blurbs (harbor, krampus,
   first-address, release-desk first — they're "open").
2. Dispatch bodies: 3 published posts need full bodies (or confirm excerpts suffice).
3. Releases: anything actually shippable? (All 6 currently unpublished.)
4. New dispatches: what's happened since Aug 18 worth a dispatch?

## Do NOT
- Fabricate post bodies, release versions, or district blurbs.
- Publish anything without his voice pass (pipeline enforces).
