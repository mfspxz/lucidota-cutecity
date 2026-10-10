//! `lucidota-receipt/1` envelope for guard proof events.
//!
//! Same transport shape as `proof-ledger` (`RECEIPT-ENVELOPE.md`): fixed
//! fields, canonical JSON (keys sorted, compact, integers only), sha256
//! `payload_hash` and `seal`, hash-chained via `prev_hash`.
//!
//! v0.1 is **unsigned**: `seal` is a plain sha256. HMAC signing waits on the
//! authority-key custody decision (Zack's call). The envelope structure is
//! identical either way, so signing upgrades without migration.

use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

pub const ENVELOPE_VERSION: &str = "lucidota-receipt/1";
pub const GENESIS: &str = "GENESIS";

/// Ternary receipt standing. Disjoint by construction — outcome lives in
/// the payload, never in the status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptStatus {
    /// Intent recorded, terminal observation absent (shift opened, tour started).
    Claimed,
    /// Terminal observation present, chain intact (check-in accepted OR
    /// rejected — a failed scan is still an established observation).
    Established,
    /// Sealing authority endorsed (supervisor sign-off on an incident).
    Authorized,
}

impl ReceiptStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ReceiptStatus::Claimed => "claimed",
            ReceiptStatus::Established => "established",
            ReceiptStatus::Authorized => "authorized",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "claimed" => Some(ReceiptStatus::Claimed),
            "established" => Some(ReceiptStatus::Established),
            "authorized" => Some(ReceiptStatus::Authorized),
            _ => None,
        }
    }
}

/// Canonical JSON: objects key-sorted (serde_json::Map is BTreeMap-backed),
/// compact, integers only. Mirrors proof-ledger `canon.rs`.
fn canonical_value(v: &Value) -> Result<Value, String> {
    match v {
        Value::Object(m) => {
            let mut sorted = Map::new();
            for (k, val) in m {
                sorted.insert(k.clone(), canonical_value(val)?);
            }
            Ok(Value::Object(sorted))
        }
        Value::Array(a) => a.iter().map(canonical_value).collect::<Result<Vec<_>, _>>().map(Value::Array),
        Value::Number(n) => {
            if n.is_f64() {
                Err("canonical: floats are not permitted in receipts".into())
            } else if n.as_i64().is_none() {
                Err("canonical: integer outside i64 interop range".into())
            } else {
                Ok(v.clone())
            }
        }
        _ => Ok(v.clone()),
    }
}

fn canonical(v: &Value) -> Result<String, String> {
    let c = canonical_value(v)?;
    serde_json::to_string(&c).map_err(|e| e.to_string())
}

fn sha256_hex(s: &str) -> String {
    hex_of(Sha256::digest(s.as_bytes()))
}

fn hex_of(digest: impl AsRef<[u8]>) -> String {
    digest.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

fn get_str(env: &Map<String, Value>, k: &str) -> Result<String, String> {
    env.get(k)
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("envelope: missing/invalid field `{k}`"))
}

/// Seal a payload into a `lucidota-receipt/1` envelope.
///
/// `ts` is RFC3339 UTC. `prev_hash` is the previous envelope's `seal`,
/// or `GENESIS` to open a chain.
pub fn seal_envelope(
    actor: &str,
    claim_id: &str,
    status: ReceiptStatus,
    payload: &Map<String, Value>,
    prev_hash: &str,
    ts: &str,
) -> Result<Map<String, Value>, String> {
    let payload_value = Value::Object(payload.clone());
    let payload_canon = canonical(&payload_value)?;
    let payload_hash = sha256_hex(&payload_canon);

    let mut env = Map::new();
    env.insert("env".into(), Value::String(ENVELOPE_VERSION.into()));
    env.insert("ts".into(), Value::String(ts.into()));
    env.insert("actor".into(), Value::String(actor.into()));
    env.insert("claim_id".into(), Value::String(claim_id.into()));
    env.insert("status".into(), Value::String(status.as_str().into()));
    env.insert("payload".into(), payload_value);
    env.insert("payload_hash".into(), Value::String(payload_hash));
    env.insert("prev_hash".into(), Value::String(prev_hash.into()));

    let seal = sha256_hex(&canonical(&Value::Object(env.clone()))?);
    env.insert("seal".into(), Value::String(seal));
    Ok(env)
}

/// Verify one envelope: version, status vocabulary, payload hash, seal.
pub fn verify_envelope(env: &Map<String, Value>) -> Result<(), String> {
    if get_str(env, "env")? != ENVELOPE_VERSION {
        return Err(format!("envelope: wrong version (want {ENVELOPE_VERSION})"));
    }
    if ReceiptStatus::from_str(&get_str(env, "status")?).is_none() {
        return Err("envelope: status outside ternary vocabulary".into());
    }
    let payload = env
        .get("payload")
        .ok_or("envelope: missing `payload`")?;
    let expect_hash = sha256_hex(&canonical(payload)?);
    if get_str(env, "payload_hash")? != expect_hash {
        return Err("envelope: payload_hash mismatch".into());
    }
    let mut nosig = env.clone();
    nosig.remove("seal");
    let expect_seal = sha256_hex(&canonical(&Value::Object(nosig))?);
    if get_str(env, "seal")? != expect_seal {
        return Err("envelope: seal mismatch".into());
    }
    Ok(())
}

/// Verify a chain: every envelope valid, every `prev_hash` links.
pub fn verify_chain(envs: &[Map<String, Value>]) -> Result<(), String> {
    let mut prev = GENESIS.to_string();
    for (i, env) in envs.iter().enumerate() {
        verify_envelope(env).map_err(|e| format!("chain[{i}]: {e}"))?;
        let ph = get_str(env, "prev_hash")?;
        if ph != prev {
            return Err(format!("chain[{i}]: prev_hash link broken"));
        }
        prev = get_str(env, "seal")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> Map<String, Value> {
        let mut p = Map::new();
        p.insert("checkpoint_id".into(), Value::String("cp-1".into()));
        p.insert("outcome".into(), Value::String("accepted".into()));
        p.insert("ts".into(), Value::Number(1_789_000_000.into()));
        p
    }

    #[test]
    fn seal_verify_roundtrip() {
        let env = seal_envelope("guard:g-9", "checkin-1", ReceiptStatus::Established, &payload(), GENESIS, "2026-10-09T13:00:00Z").unwrap();
        assert!(verify_envelope(&env).is_ok());
        assert_eq!(get_str(&env, "env").unwrap(), "lucidota-receipt/1");
    }

    #[test]
    fn tampered_payload_fails() {
        let mut env = seal_envelope("guard:g-9", "checkin-1", ReceiptStatus::Established, &payload(), GENESIS, "2026-10-09T13:00:00Z").unwrap();
        if let Some(Value::Object(p)) = env.get_mut("payload") {
            p.insert("outcome".into(), Value::String("accepted-lol".into()));
        }
        assert!(verify_envelope(&env).is_err());
    }

    #[test]
    fn chain_of_three_verifies_and_breaks_on_tamper() {
        let e1 = seal_envelope("guard:g-9", "shift-open", ReceiptStatus::Claimed, &payload(), GENESIS, "2026-10-09T12:00:00Z").unwrap();
        let s1 = get_str(&e1, "seal").unwrap();
        let e2 = seal_envelope("guard:g-9", "checkin-1", ReceiptStatus::Established, &payload(), &s1, "2026-10-09T13:00:00Z").unwrap();
        let s2 = get_str(&e2, "seal").unwrap();
        let e3 = seal_envelope("supervisor:s-2", "incident-7", ReceiptStatus::Authorized, &payload(), &s2, "2026-10-09T14:00:00Z").unwrap();
        assert!(verify_chain(&[e1.clone(), e2.clone(), e3.clone()]).is_ok());
        // Break the middle link.
        let mut bad = e2.clone();
        bad.insert("prev_hash".into(), Value::String("deadbeef".into()));
        assert!(verify_chain(&[e1, bad, e3]).is_err());
    }

    #[test]
    fn floats_rejected_from_payload() {
        let mut p = Map::new();
        p.insert("lat".into(), serde_json::json!(49.2827));
        assert!(seal_envelope("a", "c", ReceiptStatus::Claimed, &p, GENESIS, "2026-10-09T13:00:00Z").is_err());
    }

    #[test]
    fn status_vocabulary_is_ternary() {
        assert!(ReceiptStatus::from_str("claimed").is_some());
        assert!(ReceiptStatus::from_str("established").is_some());
        assert!(ReceiptStatus::from_str("authorized").is_some());
        assert!(ReceiptStatus::from_str("success").is_none());
    }
}
