# CuteCity Launch Checklist (city-014)

**D3 launch criteria. Nothing goes live without every box checked.
Voice pass (city-013) is a hard gate — the checklist enforces it mechanically.**

## How to run

```
python3 ~/workspace/cutecity/launch-check.py   # exits 0 only if ALL green
```

## Criteria

### 1. Routes (city-006) — automated
- [ ] `cute-routes verify --base https://cutecity.cc` → 29/29, exit 0
- [ ] 13 public routes 200, /nope 404, admin routes fail-closed

### 2. Admin hardening (city-007) — automated
- [ ] All 9 `/api/admin/*` routes 401/403/405 without key (in verify)
- [ ] `admin_audit` migration applied to D1 (post-freeze)
- [ ] No admin key in any file, log, or chat

### 3. Feed + public API (city-002, city-016) — automated
- [ ] `staging/feed.json` validates as JSON Feed 1.1
- [ ] `staging/v1/*.json` all parse, `version == "v1"`
- [ ] Voice-pass: `voice-pass.py publish feed` → CLEARED

### 4. Content (city-003, city-004, city-005, city-015)
- [ ] D1 content refreshed (Zackary writes — his call, not mine)
- [ ] Portfolio/blog/directory staged + voice-passed

### 5. Media (city-008)
- [ ] R2 bucket private (no public routes)
- [ ] Presigned URL round-trip verified in credentialed env (blocked in sandbox)

### 6. Subdomains (city-009, city-010, city-011)
- [ ] saltspringslasher content staged + voice-passed
- [ ] tricycles content staged + voice-passed (DNS route: CITY-1 done)
- [ ] 3 new district scaffolds staged + voice-passed

### 7. Odin C2 map (city-012)
- [ ] Frontend staged, private-by-default (secret gate, no public route)

### 8. SEO / social (city-023) — automated
- [ ] `staging/og/*.html` present for all public pages
- [ ] `staging/sitemap.xml` valid XML, all URLs 200

### 9. Accessibility (city-024)
- [ ] Static audit clean on staged templates
- [ ] Full WCAG AA pass scheduled post-launch (needs browser)

### 10. Performance (city-018)
- [ ] TTFB measured and recorded (current: 0.8–3.0s; /portfolio worst)
- [ ] LCP budget: static artifacts < 2s (worker SSR is the bottleneck — rebuild)

### 11. Analytics (city-019)
- [ ] `page_views` migration ready (aggregate-only, no PII)
- [ ] Beacon spec reviewed — no IP/UA/fingerprint columns

### 12. Backup (city-020) — automated
- [ ] `cute-backup dump-schema` OK
- [ ] `cute-backup restore-check` 0 failures

### 13. Staging parity (city-021)
- [ ] `staging/` artifacts match D1 snapshot (regenerate + diff)

### 14. Monitoring (city-022)
- [ ] Uptime cron installed, alerting to ledger

### 15. Voice pass (city-013) — THE GATE
- [ ] Every area `publish` → CLEARED (feed, og, saltspringslasher, tricycles,
      districts, odin-map, copy). Any REFUSED blocks launch.

## Current status (2026-10-07 ~05:10 UTC)

| # | Criterion | Status |
|---|-----------|--------|
| 1 | Routes | ✅ 29/29 green |
| 2 | Admin | ✅ gates verified; migration staged (post-freeze apply) |
| 3 | Feed/API | ✅ artifacts staged; ⏳ voice pass pending |
| 4 | Content | ⏳ Zackary's content call (D1 stale since Aug 18) |
| 5 | Media | ✅ code+tests; ⏳ live round-trip blocked (no R2 cred in sandbox) |
| 6 | Subdomains | 🔨 scaffolding in progress |
| 7 | Odin map | 🔨 prototype in progress |
| 8 | SEO | ✅ staged; ⏳ voice pass pending |
| 9 | A11y | 🔨 audit in progress |
| 10 | Perf | ✅ measured; worker SSR is bottleneck (rebuild needed) |
| 11 | Analytics | 🔨 spec in progress |
| 12 | Backup | ✅ dump-schema OK, restore 0 failures |
| 13 | Staging | ✅ parity via snapshot |
| 14 | Monitoring | 🔨 in progress |
| 15 | Voice pass | ⏳ NOTHING approved — correct, awaiting Zackary |
