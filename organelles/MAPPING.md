# cutecity — organelle mapping decisions

CuteCity site repo (`~/workspace/cutecity`). Five check/gate scripts, each an
independently callable capability. Prior wave-5 pass stamped all five with one
shared codon `cc` (host-registry collision); fixed here with distinct codons.

## Capability units found

| Unit | Script | Organelle op | Decision |
|---|---|---|---|
| cutecity-a11y-audit (`ay`) | `a11y-audit.py` | `audit_a11y` | **Bespoke.** Static accessibility audit (city-024): fetches public pages, runs structural checks. The fetch is incidental — the contract is an audit verdict, not `ref-scrape-extract` (which extracts content; this judges structure). |
| cutecity-launch-check (`lc`) | `launch-check.py` | `check_launch` | **Bespoke.** Launch checklist runner (city-014): exits 0 only if ALL gates green; PENDING/BLOCKED reported honestly. A release gate, not a generic validator. |
| cutecity-tipjar-check (`tj`) | `tipjar-check.py` | `check_tipjar` | **Bespoke.** Solana tip-jar wiring verifier (city-026), fail-closed. Chain-specific verification; no ref organelle touches Solana. |
| cutecity-uptime-check (`up`) | `uptime.py` | `check_uptime` | **Bespoke.** Cron-ready health probe (city-022): probes /api/health + / on cutecity.cc, records to the ledger on state CHANGE. Monitoring contract; the ledger write could delegate to `ref-ledger-append` at the host layer, but the unit's capability is the probe, not the append. |
| cutecity-voice-pass (`vp`) | `voice-pass.py` | `check_voice` | **Bespoke.** Voice-pass pipeline CLI (city-013): fail-closed staging→prod gate for site copy. Content-gating workflow; no ref organelle fits. |

## Reference-organelle check (all 12 considered, none adopted)

- `ref-scrape-extract`: a11y-audit fetches pages but its contract is an audit
  verdict, not extraction. Different computation.
- `ref-http-fetch`: uptime probes endpoints, but the contract is
  change-detect + ledger record, not fetching.
- `ref-ledger-append`: uptime *writes* ledger entries on change — a future
  host-layer delegation, not the unit's identity.
- `ref-notify`: uptime "alerts = ledger entries" — internal, not a notify op.
- Others: no overlap. Verdict: all five genuinely bespoke.

## Deliberate exclusions

- `voice-pass/` (dir): staging area for voice-passed copy — data, not code.
- `staging/`, `hidden/`: staging/data dirs.
- `rust/`: Rust workspace (cute-guard etc.) — Tier A territory, separate wave.
- `cute-city-worker-download.bin`: binary blob.
- `d1-snapshot-2026-10-07.json`, `route-verify-2026-10-07.json`: data snapshots.
- `CUTECITY-ORGANELLES.md`: earlier notes doc (superseded by this mapping).

## Execution facts

- Valence: Candidate (network-touching checks, unverified in this harness).
  All ops ReadOnly except voice-pass stage/promote steps (Write, capability-gated).
- network: egress-allowlist (cutecity.cc health/audit targets; Solana RPC for tipjar).
- kill: checks are short-lived and idempotent; suspend = drain, kill = drop,
  final receipt from the adapter.
