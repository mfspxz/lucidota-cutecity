# LANE F — CuteCity Site: Ship Report 2026-10-07

## Before
- `GET /api/health` → `{"status":"degraded","database":true,"assetStorage":false}`
- Root cause (known): `cute-city` worker had only a D1 binding (`DB`); no R2 binding, so `env.BUCKET` was undefined and every asset-storage path degraded.
- Repair previously BLOCKED on token scope (R2 v4 API → 403).

## What shipped
1. **Recovered the "lost" worker source.** Downloaded the live `cute-city` worker via the Workers API (1.6MB vinext SSR bundle, 15 modules). The bundle confirms `env.BUCKET` is the expected binding name (health check, CMS wallet-QR upload, public bundle asset serving).
2. **Found the R2 bucket.** `~/workspace/state/cloudflare.md` records bucket `cute-city-production` with working S3-API credentials. The v4 API 403 is a scope gap, not absence.
3. **Re-uploaded `cute-city` with the R2 binding added.** Same 15 modules, new metadata: D1 `DB` binding preserved + `{"type":"r2_bucket","name":"BUCKET","bucket_name":"cute-city-production"}`. (First attempt used wrong type `r2`; API error 10021 pointed to `r2_bucket`.)
4. **Deployed 2026-10-07T03:30:43Z.** No code changes, no new routes, no public surface added.

## After (dogfood, live)
- `GET /api/health` → `{"status":"ok","database":true,"assetStorage":true}` ✓
- `GET /` → 200 ✓
- `POST /api/admin/import-github` (no key) → 401 `{"error":"unauthorized"}` ✓ (gated)
- `GET /api/admin` → 404 (not exposed) ✓
- `https://saltspringslasher.cutecity.cc/` → 200 ✓
- `https://www.cutecity.cc/` → does not resolve (apex only, by design) ✓

## Exchange-routed audit (parallel probes as jobs)
Registered 4 `http-endpoint` resources and ran the evidence gauntlet + `http-check` jobs through the Exchange governor:
- `lanef-root` (cutecity.cc/) → AVAILABLE, job ok
- `lanef-admin404` (/api/admin) → AVAILABLE, job ok (404 = not exposed, correct)
- `lanef-slasher` (saltspringslasher) → AVAILABLE, job ok
- `lanef-feed` (/api/feed) → UNKNOWN; probe got HTTP 404 while direct curls intermittently return the SPA shell. **Finding:** vinext SPA fallback is inconsistent on `/api/*` paths (behavior varies with Accept headers). Not fixable without source rebuild; no public content API exists — the SPA has no JSON feed endpoint.

## D1 audit (`cute-city-production` DB 57bcfe89…)
- posts: 3 (latest 2026-08-18), releases: 6, portfolio_projects: 3, newsletter_signups: 1, site_settings: 1, ledger: 427 rows, district_overrides: 8
- Content is ~7 weeks stale — Zackary's call to update, not mine to fabricate.

## Notes / out of scope
- `tricycles` worker exists but `tricycles.cutecity.cc` has no DNS route (000). Flagged, not touched (separate lane).
- Privacy directive honored: no new public routes; admin stays 401-gated; R2 bucket remains private (binding ≠ public access).
- $0 spent. No paid services, no external actions.

## Files
- Recovered worker bundle: `~/workspace/cutecity/cute-city-worker-download.bin` (1.6MB)
- Audit probe resources: `lanef-root`, `lanef-admin404`, `lanef-slasher`, `lanef-feed` (Exchange)
