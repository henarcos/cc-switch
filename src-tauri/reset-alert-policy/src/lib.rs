use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize)]
pub struct Credit {
    pub id: String,
    pub status: String,
    #[serde(default, deserialize_with = "optional_expiry")]
    pub expires_at: Option<DateTime<Utc>>,
}

// Match CC Switch's subscription parser: an unrecognized date is unknown,
// never a reason to discard the other credits or invent a deadline.
fn optional_expiry<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<DateTime<Utc>>, D::Error> {
    let raw = Option::<String>::deserialize(deserializer)?;
    Ok(raw
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
        .map(|d| d.with_timezone(&Utc)))
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AccountState {
    /// Initial inventory is not evidence that a grant just arrived.
    pub baseline: Option<BTreeSet<String>>,
    pub delivered: BTreeSet<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    New,
    Day,
    Hour,
}
#[derive(Clone, Debug)]
pub struct Event {
    pub credit_id: String,
    pub kind: Kind,
    pub expires_at: Option<DateTime<Utc>>,
    pub receipts: Vec<String>,
}

pub fn parse_credits(bytes: &[u8]) -> Result<Vec<Credit>, String> {
    #[derive(Deserialize)]
    struct Inventory {
        credits: Vec<Credit>,
    }
    let inventory: Inventory = serde_json::from_slice(bytes).map_err(|_| "invalid inventory")?;
    let mut ids = BTreeSet::new();
    for credit in &inventory.credits {
        if credit.id.trim().is_empty() || !ids.insert(&credit.id) {
            return Err("missing or duplicate credit identity".into());
        }
    }
    Ok(inventory.credits)
}

pub fn plan(state: &mut AccountState, credits: &[Credit], now: DateTime<Utc>) -> Vec<Event> {
    let baseline = state
        .baseline
        .get_or_insert_with(|| credits.iter().map(|c| c.id.clone()).collect());
    let mut events = Vec::new();
    for c in credits {
        if c.status != "available" || c.expires_at.is_some_and(|expiry| expiry <= now) {
            continue;
        }
        let receipt = |kind: &str| serde_json::to_string(&(&c.id, kind)).expect("string tuple");
        let new = receipt("new");
        if !baseline.contains(&c.id) && !state.delivered.contains(&new) {
            events.push(Event {
                credit_id: c.id.clone(),
                kind: Kind::New,
                expires_at: c.expires_at,
                receipts: vec![new],
            });
        }
        let Some(expiry) = c.expires_at else {
            continue;
        };
        let remaining = expiry.signed_duration_since(now).num_seconds();
        let (kind, receipts) = if remaining <= 3600 {
            (Kind::Hour, vec![receipt("1h"), receipt("24h")])
        } else if remaining <= 86400 {
            (Kind::Day, vec![receipt("24h")])
        } else {
            continue;
        };
        if !state.delivered.contains(&receipts[0]) {
            events.push(Event {
                credit_id: c.id.clone(),
                kind,
                expires_at: c.expires_at,
                receipts,
            });
        }
    }
    events
}

/// Call only after the OS accepted the notification. Failure remains retryable.
pub fn acknowledge(state: &mut AccountState, event: &Event) {
    state.delivered.extend(event.receipts.clone());
}

impl Event {
    pub fn expired(&self, now: DateTime<Utc>) -> bool {
        self.expires_at.is_some_and(|at| at <= now)
    }
}
