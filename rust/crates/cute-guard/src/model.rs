//! Domain model: sites, checkpoints, tours, guards, shifts, check-ins,
//! incidents, alerts.
//!
//! Coordinates are integer microdegrees (`lat_e6`/`lon_e6`) — PL canonical
//! JSON forbids floats, and GPS is floats. 1e-6 degree ≈ 11 cm, plenty for
//! "was the guard at the door".

use serde::{Deserialize, Serialize};

/// Integer-microdegree geographic point. No floats cross a receipt boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeoPoint {
    pub lat_e6: i64,
    pub lon_e6: i64,
}

impl GeoPoint {
    pub fn from_degrees(lat: f64, lon: f64) -> Self {
        Self {
            lat_e6: (lat * 1e6).round() as i64,
            lon_e6: (lon * 1e6).round() as i64,
        }
    }
    pub fn lat(&self) -> f64 {
        self.lat_e6 as f64 / 1e6
    }
    pub fn lon(&self) -> f64 {
        self.lon_e6 as f64 / 1e6
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Geofence {
    pub center: GeoPoint,
    pub radius_m: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Site {
    pub id: String,
    pub name: String,
    pub geofence: Option<Geofence>,
}

/// How a guard proves presence at a checkpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointKind {
    Nfc,
    Qr,
    Barcode,
    GpsGeofence,
    /// Photo proof — zero hardware, the default. Hash of the photo goes in
    /// the receipt; bytes go to private object storage via presigned URL.
    Photo,
    Manual,
}

/// What the app must do when the checkpoint is reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckpointAction {
    None,
    TakePhoto,
    OpenForm { form_id: String },
    AskException { questions: Vec<String> },
    TriggerAlert { severity: u8 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub site_id: String,
    pub kind: CheckpointKind,
    pub action: CheckpointAction,
    pub instructions: String,
    /// Expected tag payload for Nfc/Qr/Barcode kinds.
    pub tag: Option<String>,
    /// Required position for GpsGeofence kind.
    pub geofence: Option<Geofence>,
}

/// One stop on a tour: which checkpoint, and the window it must be hit in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TourStop {
    pub checkpoint_id: String,
    /// Unix seconds, inclusive.
    pub window_start_ts: i64,
    pub window_end_ts: i64,
    /// Seconds past window_end before the stop counts as missed.
    pub grace_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tour {
    pub id: String,
    pub site_id: String,
    pub name: String,
    pub stops: Vec<TourStop>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guard {
    pub id: String,
    pub display_name: String,
    /// BC Security Worker Licence number, when supplied. Never invented.
    pub licence_no: Option<String>,
}

/// The proof event. Sealed into a receipt at creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckIn {
    pub checkpoint_id: String,
    pub guard_id: String,
    pub site_id: String,
    /// Unix seconds.
    pub ts: i64,
    pub kind: CheckpointKind,
    pub location: Option<GeoPoint>,
    pub accuracy_m: Option<u32>,
    /// sha256 of the photo bytes, when the action required one.
    pub photo_sha256: Option<String>,
    /// For tag kinds: did the scanned payload match the checkpoint's tag?
    pub tag_matched: Option<bool>,
    /// accepted | rejected (+ reason lives in `note`)
    pub outcome: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: String,
    pub site_id: String,
    pub guard_id: String,
    pub ts: i64,
    /// 1 (info) .. 5 (critical)
    pub severity: u8,
    pub category: String,
    pub summary: String,
    pub photo_sha256s: Vec<String>,
    pub checkpoint_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlertKind {
    MissedCheckpoint,
    MissedTour,
    LateClockIn,
    EarlyClockOut,
    GeofenceBreach,
    Panic,
    LoneWorkerTimeout,
}

/// Machine-emitted exception. The "manage by exception" feed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub kind: AlertKind,
    pub site_id: String,
    pub guard_id: String,
    pub ts: i64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shift {
    pub guard_id: String,
    pub site_id: String,
    pub clock_in_ts: i64,
    pub clock_in_location: Option<GeoPoint>,
    pub clock_out_ts: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn microdegree_roundtrip_stays_sane() {
        // Gastown-ish: 49.2827, -123.1207
        let p = GeoPoint::from_degrees(49.2827, -123.1207);
        assert_eq!(p.lat_e6, 49_282_700);
        assert_eq!(p.lon_e6, -123_120_700);
        assert!((p.lat() - 49.2827).abs() < 1e-6);
    }

    #[test]
    fn checkin_serializes_without_floats() {
        let c = CheckIn {
            checkpoint_id: "cp-1".into(),
            guard_id: "g-9".into(),
            site_id: "s-1".into(),
            ts: 1_789_000_000,
            kind: CheckpointKind::Photo,
            location: Some(GeoPoint::from_degrees(49.2827, -123.1207)),
            accuracy_m: Some(8),
            photo_sha256: Some("ab".repeat(32)),
            tag_matched: None,
            outcome: "accepted".into(),
            note: None,
        };
        let v = serde_json::to_value(&c).unwrap();
        // Walk the JSON: no Number may be f64.
        fn no_floats(v: &serde_json::Value) {
            match v {
                serde_json::Value::Number(n) => assert!(
                    !n.is_f64(),
                    "float leaked into receipt-bound model: {n}"
                ),
                serde_json::Value::Array(a) => a.iter().for_each(no_floats),
                serde_json::Value::Object(m) => m.values().for_each(no_floats),
                _ => {}
            }
        }
        no_floats(&v);
    }
}
