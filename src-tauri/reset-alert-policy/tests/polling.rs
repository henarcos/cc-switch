#[path = "../../src/services/reset_alerts_io.rs"]
mod storage;
use std::time::Duration;
#[test]
fn diagnostic_write_failure_does_not_stop_polling() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("missing/status.json");
    let wait = storage::finish_cycle(
        &path,
        &serde_json::json!({"healthy":true}),
        Duration::from_secs(20),
    );
    assert_eq!(wait, Duration::from_secs(280));
}
#[test]
fn slow_cycle_does_not_add_another_five_minute_gap() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("status.json");
    assert_eq!(
        storage::finish_cycle(&path, &true, Duration::from_secs(310)),
        Duration::ZERO
    );
}
#[test]
fn failed_ledger_write_is_still_fatal_and_never_clobbers_old_data() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("ledger.json");
    storage::save(&path, &vec!["receipt"]).unwrap();
    assert!(storage::save(&path.join("child"), &false).is_err());
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        "[\n  \"receipt\"\n]"
    );
}
