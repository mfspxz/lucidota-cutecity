# Private Analytics (city-019)

**Privacy-respecting, self-hosted, aggregate-only. No IP, no user-agent, no
fingerprint, no cookies, no cross-page identity. If it can identify a person,
it is not collected.**

## Schema (D1 migration 003 — staged in `cute-cms::schema`)

```sql
CREATE TABLE IF NOT EXISTS page_views (
  path TEXT NOT NULL,
  bucket TEXT NOT NULL,          -- hourly bucket: YYYY-MM-DDTHH
  views INTEGER NOT NULL DEFAULT 1,
  PRIMARY KEY (path, bucket)
);
```

The worker increments `views` for `(path, hour-bucket)` on each page serve.
No other columns exist — PII is structurally impossible, not just policy.

## Beacon spec (for the worker rebuild)

The current deployed worker is a compiled bundle; the beacon ships with the
rebuild. Until then, analytics is schema-ready, not live.

- Server-side counting at serve time (no client JS beacon, no third-party).
- Path normalized: strip query strings, lowercase, collapse trailing slash.
- Bot filtering: well-known bot UAs excluded from increments (list in worker
  config, not in D1).
- Admin dashboard query (secret-gated): top paths per day, from `page_views`
  only. No drill-down to individuals — there is nothing to drill into.

## What we will NOT do

- Cloudflare Web Analytics is currently injected on the live site (beacon.min.js
  observed 2026-10-07). That is Cloudflare's own analytics, not ours; our
  aggregate table replaces the need for it on the rebuild.
- No A/B testing infra, no funnels, no session replay. The question analytics
  answers is only: "which pages get visited, roughly when."

## Verification (post-rebuild)

```
-- top pages yesterday (admin only)
SELECT path, SUM(views) FROM page_views
WHERE bucket >= strftime('%Y-%m-%dT%H','now','-1 day')
GROUP BY path ORDER BY 2 DESC LIMIT 20;
-- PII absence proof: PRAGMA table_info(page_views) must show exactly
-- (path, bucket, views) — any fourth column fails the check.
```
