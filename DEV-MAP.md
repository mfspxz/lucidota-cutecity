# Cute City Dev Map — the factory IS the product's engine

Zack's directive (2026-10-07): all harness/factory work rolls into Cute City dev.
There is no separate "tentacle city project." There is Cute City, and the
engineering factory that builds it.

## The mapping

| Factory concept | Cute City product track | Current state |
|---|---|---|
| Compute seeking (DISCOVER → VERIFY) | Exchange on cutecity.workers.dev — finds and verifies free compute | v0.6.11 local, v0.6.6 deployed; D1-limited until reset |
| Governor (deterministic allocation) | Cute City's orchestration layer — decides what runs where | ESTABLISHED (court 11/11) |
| Sharding | Work distribution across workers/providers | Exchange claims model; needs product wiring |
| Presence | ODIN console, status workers, heartbeat | ctcc-router live; tentacle-city deleted per privacy directive |
| Sensors | OwnTracks ingest, edge heartbeat, data collection | ctcc-router heartbeat live; OwnTracks pending |
| Expansion | New deployments, new subdomains, new providers | Blocked on D1 reset; provider discovery via Exchange |
| Ledger/Evidence | Cute City's audit backbone — every action receipted | cutecity-ledger worker (URL unverified) |
| Verifier lanes | Trust/verification for Cute City claims | Lanes A-H verified; Lane F flag off |

## What this changes

1. No more "factory vs product" split in planning. Every factory improvement
   is evaluated by: does it make Cute City ship faster / run cheaper / verify stronger?
2. The war campaign's build order now feeds Cute City dev tracks directly.
3. The Cute City worker rebuild (WAVE-D) is the first unified build: product
   surface + factory fixes in one deploy.

## Active waves (all Cute City dev now)

- WAVE-D: Cute City worker rebuild (bundle → source, 3 fixes, deploy)
- D1 burn trace: post-reset, protects the Exchange = protects Cute City's compute
- Lane F: trust layer for Cute City claims
- Merge pipeline: factory throughput = Cute City shipping speed
