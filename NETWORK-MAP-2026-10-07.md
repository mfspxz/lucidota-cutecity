# CuteCity Network Map — 2026-10-07 (CITY-1 workstream B)

Probed 2026-10-07 ~03:50–04:10 UTC (Oct 6 ~20:50–21:10 PDT). Methods: crt.sh cert
transparency, DNS-over-HTTPS against authoritative Cloudflare DNS (sandbox UDP/53
is intercepted — see gaps), HTTPS probes with real browser User-Agent
(edge 403s bare urllib). Trinary: every claim below is labeled proven /
observed / unknown.

## DNS zone (PROVEN via DoH, authoritative)

- **Zone:** `cutecity.cc`
- **NS:** `alice.ns.cloudflare.com.`, `curt.ns.cloudflare.com.` (Cloudflare DNS)
- **Apex A:** `104.21.80.130`, `172.67.181.250` (Cloudflare anycast, proxied)
- **Apex AAAA:** `2606:4700:3034::ac43:b5fa`, `2606:4700:3031::6815:5082`
- **TXT:** `v=spf1 include:_spf.mx.cloudflare.net ~all`
- **MX:** `route2.mx.cloudflare.net.` (42), `route1.mx.cloudflare.net.` (71),
  `route3.mx.cloudflare.net.` (88) — Cloudflare Email Routing
  → handled by `cutecity-email-forwarder` worker
- **No wildcard DNS record** (random name → NXDOMAIN). Wildcard *certs* exist
  (see below) but wildcard *DNS* does not.

### Live subdomains (PROVEN)

| Subdomain | DNS | HTTP | Serves |
|---|---|---|---|
| `cutecity.cc` (apex) | A/AAAA → CF anycast | 200 `Server: cloudflare`, title "Cute City — The First Address" | `cute-city` worker (vinext SSR bundle, 1.6MB) |
| `cutecity.cc/api/health` | — | 200 JSON `{"status":"ok","database":true,"assetStorage":true}` | D1 + R2 both healthy |
| `saltspringslasher.cutecity.cc` | A → CF anycast | 200, title "Salt Spring Slasher 15 — Aug 22–23, 2026" | `saltspring-slasher-2026` worker |

### NXDOMAIN — no DNS route (PROVEN via DoH Status 3)

`www`, `tricycles`, `heartbeat`, `ai-router`, `splash`, `mail`, `api`, `dev`,
`staging`, `test`, `cdn`, `static`, `assets`, `blog`, `status`, `vpn`, `git`,
`admin`, `portal`, `app`, `exchange`, `router`, `heartbeat2`, `workers`.
24 names swept + apex. Sweep covered A records only (see gaps).

Notable: **`tricycles.cutecity.cc` NXDOMAIN** — worker exists (401 on workers.dev)
but has no DNS route (matches LANE F finding). **`heartbeat.cutecity.cc`
NXDOMAIN** — planned APK endpoint, not provisioned. **`www` NXDOMAIN** — by design
(apex only).

## Cert transparency (crt.sh, PROVEN)

Only 6 name entries — all wildcards, so crt.sh cannot enumerate individual
subdomains (cert coverage ≠ DNS existence):

- `*.cutecity.cc`, `cutecity.cc`
- `*.saltspringslasher.cutecity.cc`, `saltspringslasher.cutecity.cc`
- `*.www.cutecity.cc`, `www.cutecity.cc`

## workers.dev fleet (PROVEN, probed live)

Account subdomain: `cutecity.workers.dev`

| Worker | Probe | Verdict |
|---|---|---|
| `tricycles` | `/` → **401** "Private. Nothing to see here." | LIVE, secret-gated, no custom DNS route |
| `ai-router` | `/` → **401** `{"error":{"message":"unauthorized","type":"router_error"}}` | LIVE, ROUTER_KEY-gated (Zackary custody) |
| `ctcc-router` | `/` → 404 `{"ok":false,"error":"not found"}`; `/health` → **401** gated | LIVE (root 404 = no route there, correct) |
| `tentacle-ai-router` | `/` → **503** "router key not configured — set ROUTER_KEY secret" | LIVE, awaiting ROUTER_KEY |
| `lucidota-exchange` | `/` → **200** `{"name":"lucidota-exchange","version":"0.6.6","truth":"no evidence = UNKNOWN"}` | LIVE, Exchange governor v0.6.6 |
| `tentacle-city` | `/` and `/health` → **404** `error code: 1042` | ⚠️ workers.dev route GONE — deployed + verified live 2026-10-06 03:13 UTC; now 1042 (route detached or worker deleted). Needs dashboard check. |
| `cute-city` | 1042 | Expected — custom domain `cutecity.cc` only, no workers.dev route |
| `saltspring-slasher-2026` | 1042 | Expected — custom domain only |
| `cutecity-email-forwarder` | `/` → **500** `error code: 1101` | Email-routing worker; 1101 = worker threw on HTTP GET (not an HTTP service — expected shape, but uncaught exception, not a clean 404) |

## Bindings & data plane (from ledger + state files, PROVEN unless noted)

- **`cute-city`** — D1 `DB` → `cute-city-production`; R2 `BUCKET` →
  bucket `cute-city-production` (binding added 2026-10-07, health now ok).
- **`ctcc-router`** — D1 `heartbeat-log` (migration 0001 applied); KV `CTCC_KV`;
  secrets `HEARTBEAT_SECRET` + `CTCC_WEBHOOK_SECRET` (Zackary holds).
- **`ai-router`** — KV `LANES_KV` (lane registry); secret `ROUTER_KEY`
  (Zackary custody; vault `custom.ai-router` surrogate for agents).
- **`tentacle-ai-router`** — awaiting `ROUTER_KEY` secret (503 proves unset).
- **`lucidota-exchange`** — governor v0.6.6. D1/KV bindings: UNKNOWN (not in
  ledger excerpts inspected).
- **`cutecity-ledger`** (ledger/git wave, 2026-10-06) — D1 `LEDGER_DB` →
  `cute-city-production.ledger`; secret `LEDGER_KEY` (Zackary hands).
  Intended route `cutecity.cc/ledger/*` — **not provisioned** (v4 token 403s on
  zone workers/routes; needs dashboard or broader token).
- **`cutecity-git`** — KV `GIT_KV` → namespace `CUTECITY_GIT`; secret `GIT_KEY`.
  Intended route `cutecity.cc/git/*` — **not provisioned** (same blocker).
- **`tricycles`** — secret-gated (401 proven); bindings UNKNOWN from this probe.
- **R2** — bucket `cute-city-production`, private (presigned URLs only).
  R2 v4 API → 403 is a **scope gap, not absence** (S3-API creds verified working).

## Honest gaps

1. **R2 inventory blind** — v4 API token 403s; cannot list buckets beyond the one
   known. Scope gap, not disproven.
2. **DNS sweep was A-record only** — did not sweep CNAME/TXT/SRV per subdomain;
   a CNAME-only or TXT-only host would be missed. crt.sh wildcards mean CT can't
   close this either. Mitigation: dashboard zone read (blocked on token scope).
3. **`tentacle-city` 1042 unexplained** — route loss sometime between 2026-10-06
   03:13 UTC (verified live) and this probe. Unknown cause: redeploy with
   workers_dev disabled, worker deleted, or account-level change. Needs
   Cloudflare dashboard or v4 Workers API read.
4. **`cutecity-email-forwarder` 1101** — 500 on HTTP GET is the expected shape
   for an email worker, but 1101 is an *uncaught exception*, not a clean reject.
   Whether MX→worker email routing actually delivers is UNVERIFIED (would need a
   test email; not sent — outward action, Zackary's call).
5. **Sandbox DNS is fabricated** — local resolver `198.19.0.1` answers all
   cutecity.cc names with `198.18.13.32–37` (benchmark space, egress-proxy
   mapping). All authoritative claims above come from DoH, not local DNS.
6. **Zone-record read blocked** — v4 token 403s on zone workers/routes API, so
   no authoritative full zone dump; DoH probing is the substitute.
7. **Exchange bindings** (`lucidota-exchange` D1/KV) and **`tricycles` bindings**
   not confirmed from sources inspected this run.
