//! Opt-in banked-credit monitor. The existing OAuth manager remains the sole token owner.
use crate::proxy::providers::codex_oauth_auth::CodexOAuthManager;
use chrono::{Local, Utc};
use reset_alert_policy::{acknowledge, parse_credits, plan, AccountState, Credit, Event, Kind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::io::AsyncWriteExt;
#[path = "reset_alerts_io.rs"]
mod storage;
use storage::save as atomic_json;

#[derive(Deserialize)]
struct Config {
    enabled: bool,
    notifier_path: PathBuf,
}
#[derive(Serialize, Deserialize)]
struct Ledger {
    version: u32,
    accounts: BTreeMap<String, AccountState>,
}
#[derive(Serialize)]
struct AccountStatus {
    account: String,
    checked_at: String,
    available: Option<usize>,
    expiries: Vec<Option<String>>,
    error: Option<&'static str>,
}

pub fn start(manager: Arc<CodexOAuthManager>, directory: PathBuf) {
    let config_path = directory.join("reset-alerts.json");
    let config = match std::fs::read(config_path) {
        Ok(bytes) => match serde_json::from_slice::<Config>(&bytes) {
            Ok(config) if config.enabled => config,
            Ok(_) => return,
            Err(_) => {
                log::error!("[ResetAlerts] Invalid configuration");
                return;
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(_) => {
            log::error!("[ResetAlerts] Cannot read configuration");
            return;
        }
    };
    tauri::async_runtime::spawn(async move {
        if run(manager, directory.clone(), config).await.is_err() {
            log::error!(
                "[ResetAlerts] Monitor stopped: cannot safely load or persist notification state"
            );
            let _ = atomic_json(
                &directory.join("reset-alerts-status.json"),
                &serde_json::json!({
                    "healthy": false, "error": "state-storage-error", "checked_at": Utc::now().to_rfc3339()
                }),
            );
        }
    });
}

async fn run(
    manager: Arc<CodexOAuthManager>,
    directory: PathBuf,
    config: Config,
) -> Result<(), ()> {
    let ledger_path = directory.join("reset-alerts-ledger.json");
    let mut ledger = match std::fs::read(&ledger_path) {
        Ok(bytes) => serde_json::from_slice::<Ledger>(&bytes).map_err(|_| ())?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ledger {
            version: 1,
            accounts: BTreeMap::new(),
        },
        Err(_) => return Err(()),
    };
    if ledger.version != 1 {
        return Err(());
    }
    // Serial cycles and the CC Switch single-instance lock give this ledger one writer.
    loop {
        let cycle_started = std::time::Instant::now();
        let notifications_authorized = notification_permission(&config.notifier_path).await;
        let mut statuses = Vec::new();
        for account in manager.list_accounts().await {
            let mut status = AccountStatus {
                account: account.login.clone(),
                checked_at: Utc::now().to_rfc3339(),
                available: None,
                expiries: Vec::new(),
                error: None,
            };
            match fetch(&manager, &account.id).await {
                Ok(credits) => {
                    let now = Utc::now();
                    let available: Vec<_> = credits
                        .iter()
                        .filter(|c| {
                            c.status == "available" && c.expires_at.is_none_or(|at| at > now)
                        })
                        .collect();
                    status.available = Some(available.len());
                    status.expiries = available
                        .iter()
                        .map(|c| c.expires_at.map(|at| at.to_rfc3339()))
                        .collect();
                    let key = fingerprint(&account.id);
                    let state = ledger.accounts.entry(key.clone()).or_default();
                    let events = plan(state, &credits, now);
                    // Persist the initial baseline before sending anything.
                    atomic_json(&ledger_path, &ledger)?;
                    for event in events {
                        if event.expired(Utc::now()) {
                            continue;
                        }
                        if notify(&config.notifier_path, &account.id, &account.login, &event).await
                        {
                            acknowledge(
                                ledger.accounts.get_mut(&key).expect("account exists"),
                                &event,
                            );
                            atomic_json(&ledger_path, &ledger)?;
                        } else {
                            status.error = Some("notification-not-accepted");
                        }
                    }
                }
                Err(error) => {
                    status.error = Some(error);
                }
            }
            statuses.push(status);
        }
        let wait = storage::finish_cycle(
            &directory.join("reset-alerts-status.json"),
            &serde_json::json!({
            "healthy": notifications_authorized && statuses.iter().all(|s| s.error.is_none()),
            "notifications_authorized": notifications_authorized,
                "checked_at": Utc::now().to_rfc3339(), "poll_seconds": 300, "accounts": statuses
            }),
            cycle_started.elapsed(),
        );
        tokio::time::sleep(wait).await;
    }
}

async fn notification_permission(executable: &Path) -> bool {
    let mut command = tokio::process::Command::new(executable);
    command
        .arg("--status")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true);
    matches!(tokio::time::timeout(Duration::from_secs(20), command.status()).await,
        Ok(Ok(status)) if status.success())
}

async fn fetch(manager: &CodexOAuthManager, account_id: &str) -> Result<Vec<Credit>, &'static str> {
    let token = manager
        .get_valid_token_for_account(account_id)
        .await
        .map_err(|_| "account-authentication-error")?;
    let workspace = manager
        .chatgpt_account_id_for_account(account_id)
        .await
        .map_err(|_| "account-workspace-error")?;
    let response = crate::proxy::http_client::get()
        .get("https://chatgpt.com/backend-api/wham/rate-limit-reset-credits")
        .bearer_auth(token)
        .header("ChatGPT-Account-Id", workspace)
        .header("User-Agent", "codex-cli")
        .header("Accept", "application/json")
        .header("OpenAI-Beta", "codex-1")
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|_| "network-error")?;
    if !response.status().is_success() {
        return Err("upstream-http-error");
    }
    let body = response.bytes().await.map_err(|_| "network-error")?;
    parse_credits(&body).map_err(|_| "invalid-credit-inventory")
}

fn fingerprint(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

async fn notify(executable: &Path, account_id: &str, label: &str, event: &Event) -> bool {
    let title = match event.kind {
        Kind::New => "Novo reset extra · Codex",
        Kind::Day => "Reset extra vence em até 24h",
        Kind::Hour => "Reset extra vence em até 1h",
    };
    let deadline = event
        .expires_at
        .map(|at| {
            at.with_timezone(&Local)
                .format("%d/%m/%Y às %H:%M %Z")
                .to_string()
        })
        .unwrap_or_else(|| "prazo não informado".into());
    let identifier = fingerprint(
        &serde_json::to_string(&(account_id, &event.receipts[0])).expect("string tuple"),
    );
    let payload = serde_json::to_vec(&serde_json::json!({
        "identifier": identifier, "title": title, "body": format!("{label}\nVencimento: {deadline}")
    }))
    .expect("notice serialization");
    let mut child = match tokio::process::Command::new(executable)
        .arg("--send")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return false,
    };
    let Some(mut stdin) = child.stdin.take() else {
        return false;
    };
    if stdin.write_all(&payload).await.is_err() {
        return false;
    }
    drop(stdin);
    matches!(tokio::time::timeout(Duration::from_secs(20), child.wait()).await, Ok(Ok(status)) if status.success())
}
