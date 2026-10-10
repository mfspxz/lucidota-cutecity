//! Tour compliance engine: due/missed/pending stops, geofence checks,
//! and manage-by-exception alert emission.
//!
//! Pure logic — no I/O, no clock. The caller supplies `now_ts`.

use crate::model::*;

/// Haversine distance in meters. f64 internally; receipt-bound values
/// stay integer (see model::GeoPoint).
pub fn distance_m(a: &GeoPoint, b: &GeoPoint) -> f64 {
    let (la1, lo1, la2, lo2) = (
        a.lat().to_radians(),
        a.lon().to_radians(),
        b.lat().to_radians(),
        b.lon().to_radians(),
    );
    let dlat = la2 - la1;
    let dlon = lo2 - lo1;
    let h = (dlat / 2.0).sin().powi(2) + la1.cos() * la2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * 6_371_000.0 * h.sqrt().asin()
}

pub fn inside_geofence(p: &GeoPoint, g: &Geofence) -> bool {
    distance_m(p, &g.center) <= g.radius_m as f64
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopState {
    Completed,
    Missed,
    Pending,
}

/// Evaluate every stop of a tour against accepted check-ins at `now_ts`.
/// A stop is missed only after window_end + grace has passed with no
/// accepted check-in — no alert noise for stops still hittable.
pub fn evaluate_tour(tour: &Tour, checkins: &[CheckIn], now_ts: i64) -> Vec<(String, StopState)> {
    tour.stops
        .iter()
        .map(|stop| {
            let done = checkins.iter().any(|c| {
                c.checkpoint_id == stop.checkpoint_id
                    && c.outcome == "accepted"
                    && c.ts >= stop.window_start_ts
                    && c.ts <= stop.window_end_ts + stop.grace_secs as i64
            });
            let state = if done {
                StopState::Completed
            } else if now_ts > stop.window_end_ts + stop.grace_secs as i64 {
                StopState::Missed
            } else {
                StopState::Pending
            };
            (stop.checkpoint_id.clone(), state)
        })
        .collect()
}

/// Emit machine alerts for missed stops. One alert per missed stop —
/// the caller dedupes against already-emitted alerts by claim id.
pub fn missed_stop_alerts(
    tour: &Tour,
    checkins: &[CheckIn],
    guard_id: &str,
    now_ts: i64,
) -> Vec<Alert> {
    evaluate_tour(tour, checkins, now_ts)
        .into_iter()
        .filter(|(_, s)| *s == StopState::Missed)
        .map(|(checkpoint_id, _)| Alert {
            kind: AlertKind::MissedCheckpoint,
            site_id: tour.site_id.clone(),
            guard_id: guard_id.to_string(),
            ts: now_ts,
            detail: format!("tour {} stop {checkpoint_id} missed (window+grace elapsed)", tour.id),
        })
        .collect()
}

/// Verify a clock-in location against the site geofence, when the site
/// has one and the device supplied a fix.
pub fn verify_clock_in(site: &Site, location: Option<&GeoPoint>) -> Result<(), String> {
    match (&site.geofence, location) {
        (Some(g), Some(p)) if !inside_geofence(p, g) => Err(format!(
            "clock-in outside site geofence ({}m from center, radius {}m)",
            distance_m(p, &g.center).round() as u64,
            g.radius_m
        )),
        _ => Ok(()),
    }
}

/// Validate a tag scan against the checkpoint's expected payload.
pub fn verify_tag(checkpoint: &Checkpoint, scanned: &str) -> bool {
    match &checkpoint.tag {
        Some(expected) => expected == scanned,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tour() -> Tour {
        Tour {
            id: "t-1".into(),
            site_id: "s-1".into(),
            name: "night round".into(),
            stops: vec![
                TourStop { checkpoint_id: "cp-a".into(), window_start_ts: 1000, window_end_ts: 2000, grace_secs: 300 },
                TourStop { checkpoint_id: "cp-b".into(), window_start_ts: 2000, window_end_ts: 3000, grace_secs: 300 },
                TourStop { checkpoint_id: "cp-c".into(), window_start_ts: 9000, window_end_ts: 9900, grace_secs: 300 },
            ],
        }
    }

    fn checkin(cp: &str, ts: i64) -> CheckIn {
        CheckIn {
            checkpoint_id: cp.into(),
            guard_id: "g-9".into(),
            site_id: "s-1".into(),
            ts,
            kind: CheckpointKind::Nfc,
            location: None,
            accuracy_m: None,
            photo_sha256: None,
            tag_matched: Some(true),
            outcome: "accepted".into(),
            note: None,
        }
    }

    #[test]
    fn states_completed_missed_pending() {
        let t = tour();
        let cis = vec![checkin("cp-a", 1500)];
        let states = evaluate_tour(&t, &cis, 5000);
        assert_eq!(states[0].1, StopState::Completed);
        assert_eq!(states[1].1, StopState::Missed); // 3000+300 < 5000
        assert_eq!(states[2].1, StopState::Pending); // window not reached
    }

    #[test]
    fn grace_period_forgives_late_scan() {
        let t = tour();
        let cis = vec![checkin("cp-b", 3200)]; // 200s past window_end, inside 300s grace
        let states = evaluate_tour(&t, &cis, 5000);
        assert_eq!(states[1].1, StopState::Completed);
    }

    #[test]
    fn rejected_checkin_does_not_complete_stop() {
        let t = tour();
        let mut c = checkin("cp-a", 1500);
        c.outcome = "rejected".into();
        c.note = Some("tag mismatch".into());
        let states = evaluate_tour(&t, &[c], 5000);
        assert_eq!(states[0].1, StopState::Missed);
    }

    #[test]
    fn missed_alerts_only_for_missed() {
        let t = tour();
        let alerts = missed_stop_alerts(&t, &[checkin("cp-a", 1500)], "g-9", 5000);
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].kind, AlertKind::MissedCheckpoint);
        assert!(alerts[0].detail.contains("cp-b"));
    }

    #[test]
    fn geofence_inside_outside() {
        let center = GeoPoint::from_degrees(49.2827, -123.1207);
        let g = Geofence { center, radius_m: 50 };
        assert!(inside_geofence(&GeoPoint::from_degrees(49.2828, -123.1207), &g));
        assert!(!inside_geofence(&GeoPoint::from_degrees(49.2900, -123.1207), &g));
    }

    #[test]
    fn clock_in_outside_geofence_rejected() {
        let site = Site {
            id: "s-1".into(),
            name: "warehouse".into(),
            geofence: Some(Geofence { center: GeoPoint::from_degrees(49.2827, -123.1207), radius_m: 50 }),
        };
        let far = GeoPoint::from_degrees(49.2900, -123.1207);
        assert!(verify_clock_in(&site, Some(&far)).is_err());
        let near = GeoPoint::from_degrees(49.2828, -123.1207);
        assert!(verify_clock_in(&site, Some(&near)).is_ok());
        assert!(verify_clock_in(&site, None).is_ok()); // no fix: don't strand the guard
    }
}
