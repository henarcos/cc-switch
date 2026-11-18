use chrono::{Duration, TimeZone, Utc};
use reset_alert_policy::*;
fn now() -> chrono::DateTime<Utc> {
    Utc.timestamp_opt(1800000000, 0).unwrap()
}
fn credit(id: &str, hours: i64) -> Credit {
    Credit {
        id: id.into(),
        status: "available".into(),
        expires_at: Some(now() + Duration::hours(hours)),
    }
}
fn initialized() -> AccountState {
    let mut s = AccountState::default();
    plan(&mut s, &[], now());
    s
}
#[test]
fn existing_credits_are_baseline_not_new_grants() {
    let mut s = AccountState::default();
    assert!(plan(&mut s, &[credit("old", 48)], now()).is_empty());
    assert!(s.baseline.as_ref().unwrap().contains("old"));
}
#[test]
fn new_credit_notifies_even_when_another_was_redeemed() {
    let mut s = AccountState::default();
    plan(&mut s, &[credit("old", 48)], now());
    let events = plan(&mut s, &[credit("new", 48)], now());
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, Kind::New);
    acknowledge(&mut s, &events[0]);
    assert!(plan(&mut s, &[credit("new", 48)], now()).is_empty());
}
#[test]
fn day_and_hour_are_independent_and_survive_restart() {
    let mut s = AccountState::default();
    let c = credit("a", 25);
    plan(&mut s, &[c.clone()], now());
    let day = plan(&mut s, &[c.clone()], now() + Duration::hours(1));
    assert_eq!(day.len(), 1);
    assert_eq!(day[0].kind, Kind::Day);
    acknowledge(&mut s, &day[0]);
    let mut s: AccountState = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
    assert!(plan(&mut s, &[c.clone()], now() + Duration::hours(2)).is_empty());
    let hour = plan(&mut s, &[c], now() + Duration::hours(24));
    assert_eq!(hour.len(), 1);
    assert_eq!(hour[0].kind, Kind::Hour);
}
#[test]
fn waking_late_sends_only_the_closest_expiry_warning() {
    let mut s = AccountState::default();
    let c = credit("a", 48);
    plan(&mut s, &[c.clone()], now());
    let events = plan(
        &mut s,
        &[c.clone()],
        now() + Duration::minutes(47 * 60 + 30),
    );
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, Kind::Hour);
    acknowledge(&mut s, &events[0]);
    assert!(plan(&mut s, &[c], now() + Duration::minutes(47 * 60 + 40)).is_empty());
}
#[test]
fn expired_redeemed_redeeming_and_unknown_do_not_notify() {
    let mut s = initialized();
    let mut used = credit("used", 1);
    used.status = "redeemed".into();
    let mut redeeming = credit("busy", 1);
    redeeming.status = "redeeming".into();
    let mut unknown = credit("unknown", 1);
    unknown.status = "other".into();
    assert!(plan(
        &mut s,
        &[credit("expired", 0), used, redeeming, unknown],
        now()
    )
    .is_empty());
}
#[test]
fn failed_delivery_is_retried_and_accounts_do_not_share_receipts() {
    let mut a = initialized();
    let mut b = initialized();
    let c = credit("same-id", 48);
    let first = plan(&mut a, &[c.clone()], now());
    assert_eq!(first.len(), 1);
    assert_eq!(plan(&mut a, &[c.clone()], now()).len(), 1);
    acknowledge(&mut a, &first[0]);
    assert_eq!(plan(&mut b, &[c], now()).len(), 1);
}
#[test]
fn unknown_expiry_still_announces_arrival_without_deadline_warning() {
    let mut s = initialized();
    let mut c = credit("a", 48);
    c.expires_at = None;
    let events = plan(&mut s, &[c], now());
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, Kind::New);
}
#[test]
fn malformed_or_duplicate_inventory_is_not_an_empty_success() {
    for raw in [
        r#"{}"#,
        r#"{"credits":[{"id":"","status":"available"}]}"#,
        r#"{"credits":[{"id":"x","status":"available"},{"id":"x","status":"available"}]}"#,
    ] {
        assert!(parse_credits(raw.as_bytes()).is_err(), "{raw}");
    }
    assert!(parse_credits(br#"{"credits":[]}"#).unwrap().is_empty());
}

#[test]
fn unrecognized_expiry_does_not_hide_other_credits() {
    let credits = parse_credits(br#"{"credits":[{"id":"valid","status":"available","expires_at":"2030-01-01T00:00:00Z"},{"id":"unknown-date","status":"available","expires_at":"not-a-date"}]}"#).unwrap();
    assert_eq!(credits.len(), 2);
    assert!(credits[0].expires_at.is_some());
    assert!(credits[1].expires_at.is_none());
}

#[test]
fn planned_notice_can_expire_while_an_earlier_notice_is_delivered() {
    let mut state = initialized();
    let c = Credit {
        id: "short".into(),
        status: "available".into(),
        expires_at: Some(now() + Duration::seconds(2)),
    };
    let events = plan(&mut state, &[c], now());
    assert!(!events[0].expired(now()));
    assert!(events[0].expired(now() + Duration::seconds(3)));
}
