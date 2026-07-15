// src/wallet/reconcile.rs
//
// Phase 3a: expected-payment reconciliation. The user records payments they
// are expecting (an amount, a reference, who from). This module matches those
// against the real received transactions read from the wallet, using both the
// memo (does it contain the reference?) and the amount, and reports confidence
// honestly: confirmed, possible, or pending. Read-only with respect to the
// blockchain; the expected list is just the user's own local notes.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use zcash_protocol::consensus::Network;

use super::history::{read_history, HistoryRow};

/// One payment the user is expecting to receive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expected {
    pub reference: String,
    pub amount_zec: f64,
    pub from: String,
    pub recorded_at: String,
}

/// Where the expected-payments file lives, per network (next to the wallet db).
fn expected_path(data_dir: &Path, network: Network) -> PathBuf {
    let name = match network {
        Network::TestNetwork => "expected_payments.testnet.json",
        _ => "expected_payments.json",
    };
    data_dir.join(name)
}

/// Load the expected-payments list (empty if the file does not exist yet).
fn load_expected(data_dir: &Path, network: Network) -> Result<Vec<Expected>> {
    let path = expected_path(data_dir, network);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("could not read {}", path.display()))?;
    let list: Vec<Expected> =
        serde_json::from_str(&text).context("expected-payments file is not valid JSON")?;
    Ok(list)
}

/// Save the expected-payments list.
fn save_expected(data_dir: &Path, network: Network, list: &[Expected]) -> Result<()> {
    std::fs::create_dir_all(data_dir).ok();
    let path = expected_path(data_dir, network);
    let text = serde_json::to_string_pretty(list).context("could not serialize expected list")?;
    std::fs::write(&path, text).with_context(|| format!("could not write {}", path.display()))?;
    Ok(())
}

fn zats_to_zec(z: i64) -> f64 {
    z as f64 / 1e8
}

/// Record a new expected payment.
pub fn add_expected(
    data_dir: &Path,
    network: Network,
    amount_zec: f64,
    reference: &str,
    from: &str,
) -> Result<()> {
    let mut list = load_expected(data_dir, network)?;
    let entry = Expected {
        reference: reference.to_string(),
        amount_zec,
        from: from.to_string(),
        recorded_at: chrono::Utc::now().format("%Y-%m-%d").to_string(),
    };
    list.push(entry);
    save_expected(data_dir, network, &list)?;
    println!(
        "  Recorded: expecting {:.8} ZEC from {} (ref \"{}\").",
        amount_zec, from, reference
    );
    Ok(())
}

/// List the currently tracked expected payments.
pub fn list_expected(data_dir: &Path, network: Network) -> Result<()> {
    let list = load_expected(data_dir, network)?;
    if list.is_empty() {
        println!();
        println!("  No expected payments recorded yet.");
        println!("  Use `expect --amount <zec> --ref <reference> --from <who>` to add one.");
        return Ok(());
    }
    println!();
    println!("  Expected payments ({})", list.len());
    println!("  {:-<60}", "");
    println!(
        "  {:<16}  {:>14}  {:<24}",
        "Reference", "Amount (ZEC)", "From"
    );
    println!("  {:-<60}", "");
    for e in &list {
        println!(
            "  {:<16}  {:>14.8}  {:<24}",
            e.reference, e.amount_zec, e.from
        );
    }
    println!("  {:-<60}", "");
    Ok(())
}

/// Confidence level of a match.
enum MatchStatus {
    Confirmed(String),         // memo + amount both matched; holds the date
    PossibleMemo(String, f64), // memo matched, amount differs; date + actual amount
    PossibleAmount(String),    // amount matched, no memo confirmation; date
    Pending,
}

/// Match one expected payment against received history rows.
fn classify(expected: &Expected, rows: &[HistoryRow]) -> MatchStatus {
    let target = expected.amount_zec;
    let reference = expected.reference.to_lowercase();

    // received rows only (positive balance delta), with their decoded memo
    for r in rows {
        if r.balance_delta < 0 {
            continue;
        }
        let amount = zats_to_zec(r.balance_delta);
        let amount_matches = (amount - target).abs() < 1e-8;
        let memo_matches = r
            .memo
            .as_ref()
            .map(|m| m.to_lowercase().contains(&reference))
            .unwrap_or(false);
        let date = r
            .time
            .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "-".to_string());

        if memo_matches && amount_matches {
            return MatchStatus::Confirmed(date);
        }
        if memo_matches && !amount_matches {
            return MatchStatus::PossibleMemo(date, amount);
        }
        if amount_matches && !memo_matches {
            return MatchStatus::PossibleAmount(date);
        }
    }
    MatchStatus::Pending
}

/// Reconcile all expected payments against the wallet's received history.
pub fn reconcile(data_dir: &Path, network: Network) -> Result<()> {
    let list = load_expected(data_dir, network)?;
    if list.is_empty() {
        println!();
        println!("  No expected payments recorded yet.");
        println!("  Use `expect --amount <zec> --ref <reference> --from <who>` to add one.");
        return Ok(());
    }
    let rows = read_history(data_dir, network)?;

    println!();
    println!("  Reconciliation");
    println!("  {:-<70}", "");

    let mut total_expected = 0.0_f64;
    let mut total_received = 0.0_f64;
    let mut outstanding = 0.0_f64;

    for e in &list {
        total_expected += e.amount_zec;
        match classify(e, &rows) {
            MatchStatus::Confirmed(date) => {
                total_received += e.amount_zec;
                println!(
                    "  [received]  {} ({:.8} ZEC) from {} on {}",
                    e.reference, e.amount_zec, e.from, date
                );
            }
            MatchStatus::PossibleMemo(date, actual) => {
                outstanding += e.amount_zec;
                println!(
                    "  [check]     {} from {}: memo matched on {} but amount differs (expected {:.8}, received {:.8})",
                    e.reference, e.from, date, e.amount_zec, actual
                );
            }
            MatchStatus::PossibleAmount(date) => {
                outstanding += e.amount_zec;
                println!(
                    "  [check]     {} ({:.8} ZEC) from {}: amount matched on {} but no memo confirmation",
                    e.reference, e.amount_zec, e.from, date
                );
            }
            MatchStatus::Pending => {
                outstanding += e.amount_zec;
                println!(
                    "  [pending]   {} ({:.8} ZEC) from {}: not yet received",
                    e.reference, e.amount_zec, e.from
                );
            }
        }
    }

    println!("  {:-<70}", "");
    println!("  Expected total    : {:.8} ZEC", total_expected);
    println!("  Confirmed received: {:.8} ZEC", total_received);
    println!("  Outstanding/check : {:.8} ZEC", outstanding);
    println!("  {:-<70}", "");
    println!("  Note: [check] means a partial match worth verifying; [received] means memo and amount both matched.");
    Ok(())
}
