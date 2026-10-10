# AUDIT-APPLICATION.md

**Repo:** ~/workspace/cutecity  
**Date:** 2026-10-10  
**Auditor:** brutal-audit rollout slice 7/8 (parent orchestrator)

## Disposition Table

| Rigor finding | Application | Status |
|---|---|---|
| R-1 | No fix-verification claims in repo; policy only | POLICY ONLY |
| R-2 | cute-guard 58/58 and route-verify 29/29 need bounds docs; queued | NEEDS FOLLOW-UP |
| R-3 | No hash-chained log present | N/A |
| R-4 | LAUNCH-CHECKLIST.md pending items are backlog; voice-pass pendings explicit | CONFORMANT |
| R-5 | No signature/denylist/strip games | N/A |
| R-6 | a11y wired today; uptime.py/tipjar-check.py flagged | PARTIAL / FLAGGED |
| R-7 | Test labeling policy documented; cute-guard labeling queued | NEEDS FOLLOW-UP |
| R-8 | route-verify and d1-snapshot JSON are unsigned artifacts | NEEDS FOLLOW-UP |
| R-9 | No claim logs | N/A |
| R-10 | SHIP-REPORT and route-verify JSON written in same motion as events | CONFORMANT |
| R-11 | Unknown list provided in this document | CONFORMANT (this doc) |
| R-12 | Dated artifacts used; no oral footnotes found | CONFORMANT |

## R-1 — Fix-verification claims

**Status:** POLICY ONLY / CONFORMANT

No red-team or fix-verification claims exist in this repo. Policy stated plainly: any future fix claim is instance-only until a dedicated re-attack signs off. Claims and defeats must live in one book. No PROVEN/FIXED without a re-attack result.

- PROVEN: none.
- DISPROVEN: none.
- UNKNOWN: no fix-verification claims present.

## R-2 — Guardrail bounds

**Status:** NEEDS FOLLOW-UP

cute-guard 58/58 tests and route-verify 29/29 need documented bounds: what the tests cover versus what they do not; route verification checks route existence, not content correctness.

Bounds were **not** written today. Queued as follow-up. Until bounds are written, no claim beyond “tests pass” is PROVEN. Hardening past the documented bound freezes.

- PROVEN: cute-guard 58/58 recorded green per 2026-10-07 commit.
- UNKNOWN: actual coverage bounds for cute-guard; exact content-correctness coverage of route-verify.

## R-3 — Hash-chained logs

**Status:** N/A

No hash-chained log exists in this repo. No tamper-evident claims present.

## R-4 — Backlog triage

**Status:** CONFORMANT

LAUNCH-CHECKLIST.md pending items are the backlog. voice-pass pendings are explicit and honest — a queue, not hidden standing HIGHs. Backlog must not grow without triage.

No standing HIGH wont-fix items present.

## R-5 — Signature/denylist/strip games

**Status:** N/A

No signature, denylist, or strip-based control games in repo.

## R-6 — Detector consumers

**Status:** PARTIAL / FLAGGED

a11y-audit.py was wired today into launch-check.py §16 as its named consumer with an explicit ADVISORY policy. a11y fetches live pages and cannot hard-block pre-launch; failures are recorded, not launch-blocking. The post-launch browser pass becomes a hard gate.

uptime.py and tipjar-check.py are monitors with no wired alerting consumer. Flagged for wiring or deletion in the same change.

- PROVEN: a11y-audit.py now has a named consumer.
- UNKNOWN: uptime.py/tipjar-check.py final consumer state.

## R-7 — Property vs change-detection tests

**Status:** NEEDS FOLLOW-UP

cute-guard 58/58 tests exist, but property vs change-detection labeling is UNKNOWN. Policy documented: separate and explicitly label property tests from change-detection tests; never rewrite a property test expectation without adversarial sign-off.

Labeling queued.

- UNKNOWN: which cute-guard tests are property tests versus change-detection tests.

## R-8 — Unsigned provenance artifacts

**Status:** NEEDS FOLLOW-UP

route-verify-2026-10-07.json and d1-snapshot-2026-10-07.json are UNSIGNED artifacts. Declare honestly-unsigned: verification power is to re-run the generator against the recorded tree, not to trust the JSON file.

No signing or anchoring exists in repo today.

## R-9 — Claim logs

**Status:** N/A

No claim logs present.

## R-10 — Record-keeping timing

**Status:** CONFORMANT

SHIP-REPORT-2026-10-07.md and route-verify JSON were written in the same motion as the events they record.

## R-11 — Explicit UNKNOWN list

**Status:** CONFORMANT (this doc)

See “UNKNOWN list” section below.

## R-12 — Unsourced oral footnotes

**Status:** CONFORMANT

BUILD-PLAN and SHIP-REPORT claims reference dated artifacts. No oral footnotes found.

## Changes Made Today

- launch-check.py §16 now wires a11y-audit.py into the launch report as its named consumer.
- a11y policy is explicit ADVISORY: failures are recorded, not launch-blocking.
- Post-launch browser accessibility pass becomes a hard gate.
- Gate advisory prefix list now includes `a11y:` alongside `voice-pass:`.

## Commits

- `55306d9` — launch-check.py §16: a11y-audit advisory wiring, named consumer, advisory prefix; AUDIT-APPLICATION.md.
- Follow-up queued items (bounds docs, uptime/tipjar wiring): NOT DONE in this pass.

## Test Counts

- cute-guard: 58/58 per 2026-10-07 commit — **recorded, not re-run this pass**.
- route-verify: 29/29 recorded from artifact — **not re-run this pass**.
- a11y-audit.py: **not run this pass**.
- Launch gate end-to-end green: **UNKNOWN — never run end-to-end green**.

## UNKNOWN List

- cute-guard actual coverage bounds: what the 58 tests prove and do not prove.
- Whether the 29/29 route-verify covers content correctness: UNKNOWN; observed behavior checks route existence, but no bounds doc states this.
- uptime.py consumer state: no wired alerting consumer documented.
- Launch gate has never been run end-to-end green.

## Wont-fix / Backlog Triage

- voice-pass pendings: honest queue, not wont-fix.
- uptime.py/tipjar-check.py wiring or deletion: QUEUED with budget.
- cute-guard bounds doc: QUEUED.
- cute-guard property/change-detection test labeling: QUEUED.
- Unsigned artifact declaration or signing/anchor: QUEUED.
