//! `gd` — the guard-tour module as an organelle.
//!
//! Same contract shape as pouw-mesh's organelle trait (the workspace
//! precedent is one minimal trait per crate: go50, krx). Every mutating op
//! seals a `lucidota-receipt/1` envelope, hash-chained through the
//! organelle's `chain_tip` — the receipts ARE the audit trail.

use crate::engine::{self, StopState};
use crate::model::*;
use crate::receipt::{self, ReceiptStatus, GENESIS};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::Mutex;

// --- contract (mirrors pouw-mesh/src/organelle.rs) ---

#[derive(Debug, Clone)]
pub struct Authority {
    pub may_read: Vec<String>,
    pub may_write: Vec<String>,
    pub may_not: Vec<String>,
    pub fail_closed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationClass {
    ReadOnly,
    Write,
    ExternalEffect,
}

impl MutationClass {
    pub fn as_str(self) -> &'static str {
        match self {
            MutationClass::ReadOnly => "ReadOnly",
            MutationClass::Write => "Write",
            MutationClass::ExternalEffect => "ExternalEffect",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Valence {
    Proven,
    Candidate,
    Tainted,
}

impl Valence {
    pub fn as_str(self) -> &'static str {
        match self {
            Valence::Proven => "Proven",
            Valence::Candidate => "Candidate",
            Valence::Tainted => "Tainted",
        }
    }
}

pub trait Organelle: Send + Sync {
    fn codon(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn call(&self, input: &str) -> Result<String, String>;
    fn authority(&self) -> Authority;
    fn mutation_class(&self) -> MutationClass;
    fn valence(&self) -> Valence;
    fn capability(&self) -> Option<&'static str>;
    fn input_schema(&self) -> Value;
    fn manifest_json(&self) -> &'static str;
}

/// Max input envelope: 1 MiB. Anything larger is a resource-exhaustion
/// probe, rejected before serde_json allocates.
const MAX_INPUT_BYTES: usize = 1 << 20;

fn parse_op(input: &str) -> Result<(String, Map<String, Value>), String> {
    if input.len() > MAX_INPUT_BYTES {
        return Err("gd: input exceeds 1 MiB cap".into());
    }
    let v: Value =
        serde_json::from_str(input).map_err(|e| format!("gd: bad JSON input: {e}"))?;
    let m = v.as_object().ok_or("gd: input must be a JSON object")?.clone();
    let op = m
        .get("op")
        .and_then(Value::as_str)
        .ok_or("gd: missing string field `op`")?;
    Ok((op.to_string(), m))
}

fn get_str(m: &Map<String, Value>, k: &str) -> Result<String, String> {
    m.get(k)
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("gd: missing/invalid string field `{k}`"))
}

fn get_i64(m: &Map<String, Value>, k: &str) -> Result<i64, String> {
    m.get(k)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("gd: missing/invalid i64 field `{k}`"))
}

fn ok_json(v: Value) -> Result<String, String> {
    serde_json::to_string(&v).map_err(|e| e.to_string())
}

// --- state ---

struct GuardState {
    sites: HashMap<String, Site>,
    checkpoints: HashMap<String, Checkpoint>,
    tours: HashMap<String, Tour>,
    checkins: Vec<CheckIn>,
    incidents: Vec<Incident>,
    shifts: Vec<Shift>,
    chain_tip: String,
}

impl GuardState {
    fn new() -> Self {
        Self {
            sites: HashMap::new(),
            checkpoints: HashMap::new(),
            tours: HashMap::new(),
            checkins: Vec::new(),
            incidents: Vec::new(),
            shifts: Vec::new(),
            chain_tip: GENESIS.to_string(),
        }
    }
}

pub struct GuardOrganelle {
    state: Mutex<GuardState>,
}

impl GuardOrganelle {
    pub fn new() -> Self {
        Self { state: Mutex::new(GuardState::new()) }
    }

    /// Seal a payload, advance the chain tip, return the envelope.
    fn seal(
        state: &mut GuardState,
        actor: &str,
        claim_id: &str,
        status: ReceiptStatus,
        payload: Map<String, Value>,
        sealed_ts: &str,
    ) -> Result<Map<String, Value>, String> {
        let env = receipt::seal_envelope(actor, claim_id, status, &payload, &state.chain_tip, sealed_ts)?;
        let seal = env
            .get("seal")
            .and_then(Value::as_str)
            .ok_or("gd: seal failed")?
            .to_string();
        state.chain_tip = seal;
        Ok(env)
    }

    fn check_in_op(state: &mut GuardState, m: &Map<String, Value>) -> Result<Value, String> {
        let mut ci: CheckIn = serde_json::from_value(
            m.get("checkin").cloned().ok_or("gd: missing field `checkin`")?,
        )
        .map_err(|e| format!("gd: bad checkin: {e}"))?;
        let sealed_ts = get_str(m, "sealed_ts")?;
        let cp = state
            .checkpoints
            .get(&ci.checkpoint_id)
            .ok_or("gd: unknown checkpoint_id")?
            .clone();

        // Tag kinds: the scanned payload must match, or the check-in is rejected.
        // A rejection is still a sealed observation (established, outcome=rejected).
        let (outcome, note) = match cp.kind {
            CheckpointKind::Nfc | CheckpointKind::Qr | CheckpointKind::Barcode => {
                let scanned = get_str(m, "scanned_tag")?;
                if engine::verify_tag(&cp, &scanned) {
                    ("accepted".to_string(), None)
                } else {
                    ("rejected".to_string(), Some("tag mismatch".to_string()))
                }
            }
            CheckpointKind::GpsGeofence => match (&cp.geofence, &ci.location) {
                (Some(g), Some(p)) if engine::inside_geofence(p, g) => ("accepted".to_string(), None),
                (Some(_), Some(_)) => ("rejected".to_string(), Some("outside checkpoint geofence".to_string())),
                _ => ("accepted".to_string(), Some("no fix; accepted without position".to_string())),
            },
            CheckpointKind::Photo | CheckpointKind::Manual => ("accepted".to_string(), None),
        };
        ci.outcome = outcome;
        ci.note = note;
        ci.tag_matched = match cp.kind {
            CheckpointKind::Nfc | CheckpointKind::Qr | CheckpointKind::Barcode => {
                Some(ci.outcome == "accepted")
            }
            _ => None,
        };

        let payload = serde_json::to_value(&ci)
            .map_err(|e| e.to_string())?
            .as_object()
            .cloned()
            .ok_or("gd: checkin not an object")?;
        let claim_id = format!("checkin-{}-{}", ci.checkpoint_id, ci.ts);
        let env = Self::seal(
            state,
            &format!("guard:{}", ci.guard_id),
            &claim_id,
            ReceiptStatus::Established,
            payload,
            &sealed_ts,
        )?;
        state.checkins.push(ci.clone());
        Ok(json!({"receipt": env, "outcome": ci.outcome}))
    }
}

impl Default for GuardOrganelle {
    fn default() -> Self {
        Self::new()
    }
}

impl Organelle for GuardOrganelle {
    fn codon(&self) -> &'static str { "gd" }
    fn name(&self) -> &'static str { "guard-tour" }
    fn version(&self) -> &'static str { "0.1.0" }

    fn call(&self, input: &str) -> Result<String, String> {
        let (op, m) = parse_op(input)?;
        let mut state = self.state.lock().map_err(|e| e.to_string())?;
        match op.as_str() {
            "register_site" => {
                let site: Site = serde_json::from_value(
                    m.get("site").cloned().ok_or("gd: missing field `site`")?,
                )
                .map_err(|e| format!("gd: bad site: {e}"))?;
                let id = site.id.clone();
                state.sites.insert(id.clone(), site);
                // Checkpoints arrive inside the site registration for throughput:
                // {"site": {...}, "checkpoints": [...]}
                if let Some(cps) = m.get("checkpoints").and_then(Value::as_array) {
                    for cp in cps {
                        let c: Checkpoint = serde_json::from_value(cp.clone())
                            .map_err(|e| format!("gd: bad checkpoint: {e}"))?;
                        state.checkpoints.insert(c.id.clone(), c);
                    }
                }
                ok_json(json!({"site_id": id}))
            }
            "define_tour" => {
                let tour: Tour = serde_json::from_value(
                    m.get("tour").cloned().ok_or("gd: missing field `tour`")?,
                )
                .map_err(|e| format!("gd: bad tour: {e}"))?;
                let id = tour.id.clone();
                state.tours.insert(id.clone(), tour);
                ok_json(json!({"tour_id": id}))
            }
            "check_in" => {
                let v = Self::check_in_op(&mut state, &m)?;
                ok_json(v)
            }
            "report_incident" => {
                let inc: Incident = serde_json::from_value(
                    m.get("incident").cloned().ok_or("gd: missing field `incident`")?,
                )
                .map_err(|e| format!("gd: bad incident: {e}"))?;
                let sealed_ts = get_str(&m, "sealed_ts")?;
                let payload = serde_json::to_value(&inc)
                    .map_err(|e| e.to_string())?
                    .as_object()
                    .cloned()
                    .ok_or("gd: incident not an object")?;
                let env = Self::seal(
                    &mut state,
                    &format!("guard:{}", inc.guard_id),
                    &inc.id,
                    ReceiptStatus::Claimed,
                    payload,
                    &sealed_ts,
                )?;
                state.incidents.push(inc.clone());
                ok_json(json!({"receipt": env, "incident_id": inc.id}))
            }
            "clock_in" => {
                let guard_id = get_str(&m, "guard_id")?;
                let site_id = get_str(&m, "site_id")?;
                let ts = get_i64(&m, "ts")?;
                let sealed_ts = get_str(&m, "sealed_ts")?;
                let site = state.sites.get(&site_id).ok_or("gd: unknown site_id")?.clone();
                let loc: Option<GeoPoint> = m
                    .get("location")
                    .map(|v| serde_json::from_value(v.clone()))
                    .transpose()
                    .map_err(|e| format!("gd: bad location: {e}"))?;
                engine::verify_clock_in(&site, loc.as_ref()).map_err(|e| format!("gd: {e}"))?;
                let shift = Shift {
                    guard_id: guard_id.clone(),
                    site_id: site_id.clone(),
                    clock_in_ts: ts,
                    clock_in_location: loc,
                    clock_out_ts: None,
                };
                let payload = serde_json::to_value(&shift)
                    .map_err(|e| e.to_string())?
                    .as_object()
                    .cloned()
                    .ok_or("gd: shift not an object")?;
                let env = Self::seal(
                    &mut state,
                    &format!("guard:{guard_id}"),
                    &format!("shift-{guard_id}-{ts}"),
                    ReceiptStatus::Claimed,
                    payload,
                    &sealed_ts,
                )?;
                state.shifts.push(shift);
                ok_json(json!({"receipt": env}))
            }
            "clock_out" => {
                let guard_id = get_str(&m, "guard_id")?;
                let ts = get_i64(&m, "ts")?;
                let sealed_ts = get_str(&m, "sealed_ts")?;
                let idx = state
                    .shifts
                    .iter()
                    .rposition(|s| s.guard_id == guard_id && s.clock_out_ts.is_none())
                    .ok_or("gd: no open shift")?;
                state.shifts[idx].clock_out_ts = Some(ts);
                let shift = state.shifts[idx].clone();
                let clock_in_ts = shift.clock_in_ts;
                let payload = serde_json::to_value(&shift)
                    .map_err(|e| e.to_string())?
                    .as_object()
                    .cloned()
                    .ok_or("gd: shift not an object")?;
                let env = Self::seal(
                    &mut state,
                    &format!("guard:{guard_id}"),
                    &format!("shift-{guard_id}-{clock_in_ts}-out"),
                    ReceiptStatus::Established,
                    payload,
                    &sealed_ts,
                )?;
                ok_json(json!({"receipt": env}))
            }
            "panic" => {
                let guard_id = get_str(&m, "guard_id")?;
                let site_id = get_str(&m, "site_id")?;
                let ts = get_i64(&m, "ts")?;
                let sealed_ts = get_str(&m, "sealed_ts")?;
                let alert = Alert {
                    kind: AlertKind::Panic,
                    site_id: site_id.clone(),
                    guard_id: guard_id.clone(),
                    ts,
                    detail: "panic button pressed".into(),
                };
                let payload = serde_json::to_value(&alert)
                    .map_err(|e| e.to_string())?
                    .as_object()
                    .cloned()
                    .ok_or("gd: alert not an object")?;
                let env = Self::seal(
                    &mut state,
                    &format!("guard:{guard_id}"),
                    &format!("panic-{guard_id}-{ts}"),
                    ReceiptStatus::Established,
                    payload,
                    &sealed_ts,
                )?;
                ok_json(json!({"receipt": env, "alert": alert}))
            }
            "tour_status" => {
                let tour = state.tours.get(&get_str(&m, "tour_id")?).ok_or("gd: unknown tour_id")?.clone();
                let now_ts = get_i64(&m, "now_ts")?;
                let stops: Vec<Value> = engine::evaluate_tour(&tour, &state.checkins, now_ts)
                    .into_iter()
                    .map(|(id, s)| {
                        let st = match s {
                            StopState::Completed => "completed",
                            StopState::Missed => "missed",
                            StopState::Pending => "pending",
                        };
                        json!({"checkpoint_id": id, "state": st})
                    })
                    .collect();
                ok_json(json!({"tour_id": tour.id, "stops": stops}))
            }
            "missed_checkpoints" => {
                let tour = state.tours.get(&get_str(&m, "tour_id")?).ok_or("gd: unknown tour_id")?.clone();
                let guard_id = get_str(&m, "guard_id")?;
                let now_ts = get_i64(&m, "now_ts")?;
                let alerts = engine::missed_stop_alerts(&tour, &state.checkins, &guard_id, now_ts);
                ok_json(json!({"alerts": alerts}))
            }
            _ => Err(format!("gd: unknown op `{op}`")),
        }
    }

    fn authority(&self) -> Authority {
        Authority {
            may_read: vec!["sites, checkpoints, tours, check-ins in organelle memory".into()],
            may_write: vec!["check-ins, incidents, shifts, alerts as sealed receipts".into()],
            may_not: vec![
                "network".into(),
                "filesystem".into(),
                "secret access".into(),
                "photo bytes (hashes only; bytes go to private storage via presigned URL)".into(),
            ],
            fail_closed: true,
        }
    }
    fn mutation_class(&self) -> MutationClass { MutationClass::Write }
    fn valence(&self) -> Valence { Valence::Candidate }
    fn capability(&self) -> Option<&'static str> { Some("guard-firm") }
    fn input_schema(&self) -> Value {
        json!({"type": "object", "required": ["op"],
               "properties": {"op": {"enum": ["register_site", "define_tour", "check_in",
                 "report_incident", "clock_in", "clock_out", "panic",
                 "tour_status", "missed_checkpoints"]}}})
    }
    fn manifest_json(&self) -> &'static str {
        include_str!("manifests/gd.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversize_input_rejected_before_parse() {
        let o = GuardOrganelle::new();
        let big = "z".repeat(2 * 1024 * 1024);
        let err = o.call(&format!(r#"{{"op": "checkin", "note": "{big}"}}"#)).unwrap_err();
        assert!(err.contains("1 MiB"), "unexpected: {err}");
    }

    fn call(o: &GuardOrganelle, v: Value) -> Value {
        let out = o.call(&v.to_string()).expect("call failed");
        serde_json::from_str(&out).unwrap()
    }

    fn site_json() -> Value {
        json!({"id": "s-1", "name": "warehouse", "geofence": null})
    }

    fn checkpoint_json() -> Value {
        json!({"id": "cp-1", "site_id": "s-1", "kind": "nfc",
               "action": "none", "instructions": "scan the tag",
               "tag": "TAG-AAA", "geofence": null})
    }

    fn photo_checkpoint_json() -> Value {
        json!({"id": "cp-photo", "site_id": "s-1", "kind": "photo",
               "action": "take_photo", "instructions": "photo the dock",
               "tag": null, "geofence": null})
    }

    fn setup() -> GuardOrganelle {
        let o = GuardOrganelle::new();
        call(&o, json!({"op": "register_site", "site": site_json(),
                        "checkpoints": [checkpoint_json(), photo_checkpoint_json()]}));
        call(&o, json!({"op": "define_tour", "tour": {
            "id": "t-1", "site_id": "s-1", "name": "round",
            "stops": [{"checkpoint_id": "cp-1", "window_start_ts": 1000,
                       "window_end_ts": 2000, "grace_secs": 300}]}}));
        o
    }

    #[test]
    fn full_patrol_cycle_through_organelle() {
        let o = setup();
        let mut chain: Vec<Map<String, Value>> = Vec::new();
        let mut take = |v: Value| {
            let r: Value = call(&o, v);
            chain.push(serde_json::from_value(r["receipt"].clone()).unwrap());
            r
        };
        // clock in
        let r = take(json!({"op": "clock_in", "guard_id": "g-9", "site_id": "s-1",
                            "ts": 900, "sealed_ts": "2026-10-09T13:00:00Z"}));
        assert!(r.get("receipt").is_some());
        // NFC check-in with the right tag
        let r = take(json!({"op": "check_in",
            "checkin": {"checkpoint_id": "cp-1", "guard_id": "g-9", "site_id": "s-1",
                        "ts": 1500, "kind": "nfc", "location": null, "accuracy_m": null,
                        "photo_sha256": null, "tag_matched": null,
                        "outcome": "pending", "note": null},
            "scanned_tag": "TAG-AAA", "sealed_ts": "2026-10-09T13:01:00Z"}));
        assert_eq!(r["outcome"], "accepted");
        assert!(r["receipt"]["seal"].as_str().is_some());
        // tour status: stop completed
        let r = call(&o, json!({"op": "tour_status", "tour_id": "t-1", "now_ts": 5000}));
        assert_eq!(r["stops"][0]["state"], "completed");
        // clock out
        let r = take(json!({"op": "clock_out", "guard_id": "g-9",
                            "ts": 8000, "sealed_ts": "2026-10-09T14:00:00Z"}));
        assert!(r.get("receipt").is_some());
        // the whole shift is one verifiable hash chain, GENESIS → tip
        assert!(receipt::verify_chain(&chain).is_ok());
        assert_eq!(chain.len(), 3);
    }

    #[test]
    fn wrong_tag_rejected_but_sealed() {
        let o = setup();
        let r = call(&o, json!({"op": "check_in",
            "checkin": {"checkpoint_id": "cp-1", "guard_id": "g-9", "site_id": "s-1",
                        "ts": 1500, "kind": "nfc", "location": null, "accuracy_m": null,
                        "photo_sha256": null, "tag_matched": null,
                        "outcome": "pending", "note": null},
            "scanned_tag": "TAG-WRONG", "sealed_ts": "2026-10-09T13:01:00Z"}));
        assert_eq!(r["outcome"], "rejected");
        // ...but the rejection is still a sealed, verifiable receipt
        let env: Map<String, Value> = serde_json::from_value(r["receipt"].clone()).unwrap();
        assert!(receipt::verify_envelope(&env).is_ok());
        assert_eq!(env["status"], "established");
    }

    #[test]
    fn photo_checkin_zero_hardware() {
        let o = setup();
        let r = call(&o, json!({"op": "check_in",
            "checkin": {"checkpoint_id": "cp-photo", "guard_id": "g-9", "site_id": "s-1",
                        "ts": 1600, "kind": "photo", "location": null, "accuracy_m": null,
                        "photo_sha256": "ab".repeat(32).as_str(), "tag_matched": null,
                        "outcome": "pending", "note": null},
            "sealed_ts": "2026-10-09T13:02:00Z"}));
        assert_eq!(r["outcome"], "accepted");
    }

    #[test]
    fn panic_and_incident() {
        let o = setup();
        let r = call(&o, json!({"op": "panic", "guard_id": "g-9", "site_id": "s-1",
                               "ts": 1700, "sealed_ts": "2026-10-09T13:03:00Z"}));
        assert_eq!(r["alert"]["kind"], "panic");
        let r = call(&o, json!({"op": "report_incident",
            "incident": {"id": "i-1", "site_id": "s-1", "guard_id": "g-9", "ts": 1750,
                         "severity": 3, "category": "trespass", "summary": "fence cut",
                         "photo_sha256s": [], "checkpoint_id": null},
            "sealed_ts": "2026-10-09T13:04:00Z"}));
        assert_eq!(r["incident_id"], "i-1");
        // incidents open as claimed (supervisor authorizes later)
        let env: Map<String, Value> = serde_json::from_value(r["receipt"].clone()).unwrap();
        assert_eq!(env["status"], "claimed");
    }

    #[test]
    fn manifest_matches_interface() {
        let o = GuardOrganelle::new();
        let mj = o.manifest_json();
        for field in ["\"codon\"", "\"name\"", "\"version\"", "\"contracts\"",
                      "\"authority\"", "\"transports\"", "\"receipts\"",
                      "\"mutation_class\"", "\"valence\""] {
            assert!(mj.contains(field), "gd manifest missing {field}");
        }
        assert!(mj.contains("\"codon\": \"gd\""));
        assert_eq!(o.codon(), "gd");
        assert_eq!(o.mutation_class(), MutationClass::Write);
        assert_eq!(o.capability(), Some("guard-firm"));
        assert!(o.authority().fail_closed);
    }
}
