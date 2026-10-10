# cute-guard — CuteCity guard-tour mother module

**Status:** v0.1 scaffold — domain model + receipt envelope + tour engine.
**Organelle codon:** `gd`. **Crate:** `cute-guard`.

## Why this exists

TrackTik (Montreal, est. 2013, now Trackforce) owns the guard-tour market with
an enterprise-priced, opaque-quote product. The wedge for a Vancouver build:

1. **Transparent per-guard pricing** (competitors publish ~$3.20/guard/mo;
   TrackTik won't show a number at all).
2. **Photo check-ins as a first-class checkpoint kind** — zero hardware, zero
   training, proof of presence. This is how Zack's own site already works.
3. **Battery-light** — geofence transitions + checkpoint scans primary;
   continuous GPS only inside active tour windows.
4. **Receipt-backed proof** — every check-in, incident, and alert is sealed in
   a `lucidota-receipt/1` envelope, hash-chained. Can't lie, can't hide.
   (Same doctrine as his aviation safety ledger: Merkle-backed records.)
5. **BC-shaped** — PIPA-conscious location handling, BC Security Worker
   Licence fields on the guard record.

## Design decisions (opinions, recorded)

- **Photo > NFC as the default.** NFC tags (the "circular magnetic things")
  are supported, but tags get ripped off walls, need stocking, and add a
  procurement step. A photo with a hash in the receipt proves presence with
  hardware every guard already carries.
- **No floats in receipts.** PL canonical JSON forbids floats, and GPS is
  floats. Coordinates are stored as integer microdegrees (`lat_e6`, `lon_e6`).
  Internal distance math may use f64; receipt bytes never do.
- **Unsigned envelopes in v0.1.** The `seal` is sha256 over the canonical
  envelope (same 9 fields, same chain semantics as `proof-ledger`). HMAC
  signing waits on the authority-key custody decision (open, Zack's call) —
  see tracked goal `authority-key-and-runner-root-custody`.
- **Manage-by-exception, like TrackTik, but dumber and faster.** The engine
  emits alerts only for: missed checkpoint (window + grace elapsed),
  missed tour, late clock-in, early clock-out, geofence breach, panic,
  lone-worker timeout. No dashboard noise.
- **Offline-first.** Check-ins seal locally into an outbox; sync when
  connected. Matches TrackTik's offline behavior — table stakes, not a
  differentiator.
- **Privacy fail-closed.** Location leaves the device only inside a sealed
  receipt to the firm's own backend. The organelle host enforces capability
  gates; this crate never touches network, keys, or storage.

## Entities

| Entity | Notes |
|---|---|
| `Site` | name + optional geofence |
| `Checkpoint` | kind (Nfc/Qr/Barcode/GpsGeofence/Photo/Manual), per-point action (None/TakePhoto/OpenForm/AskException/TriggerAlert), instructions |
| `Tour` | ordered stops, each with a time window + grace period |
| `Guard` | id + display name (+ licence fields later) |
| `Shift` | clock-in/out with geofence verification |
| `CheckIn` | the proof event; photo stored as sha256 only — bytes go to private R2 via presigned URL, never in the receipt |
| `Incident` | severity, category, summary, photo hashes |
| `Alert` | machine-emitted exception |

## Receipt status mapping (ternary, per RECEIPT-ENVELOPE.md)

- `claimed` — intent with no terminal observation yet: shift opened, tour started.
- `established` — terminal observation, chain intact: check-in accepted
  **or rejected** (a failed scan is still an established observation —
  outcome lives in the payload, per RCPT-009).
- `authorized` — supervisor endorsement axis: incident report signed off.

## Organelle ops (`gd`)

`register_site` · `define_tour` · `check_in` · `report_incident` ·
`clock_in` · `clock_out` · `panic` · `tour_status` · `missed_checkpoints`

Mutation class: Write (check-ins/incidents/alerts append receipts).
Capability: `guard-firm` for writes; none for `tour_status` reads of own site.
Valence: Candidate until the engine's golden vectors are host-verified.

## Odin integration

Odin Vancouver consumes sealed check-in/alert receipts as a map layer:
live guard positions (from check-in locations), checkpoint status pins,
alert pins. Contract: receipts only — the map never gets raw location
streams. Frontend wiring is a later wave; `tentacle-corp/odin-map` holds
the map side.

## Out of scope for v0.1

Mobile UI, server sync transport, scheduling/dispatch, payroll/billing,
client portal, HMAC signing, push notifications. Those are later waves —
this wave proves the domain model and the receipt chain.
