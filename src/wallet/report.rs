// src/wallet/report.rs
//
// Step 2b: accounting reports. Monthly summary on screen, full ledger to
// CSV and JSON files. Reads the same v_transactions view as history.

use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

use super::history::{read_history, HistoryRow};
use zcash_protocol::consensus::Network;

fn zats_to_zec(z: i64) -> f64 {
    z as f64 / 1e8
}

#[derive(Serialize)]
pub struct LedgerEntry {
    pub txid: String,
    pub height: Option<i64>,
    pub date: Option<String>,
    pub amount_zec: f64,
    pub fee_zec: f64,
    pub kind: String,
}

#[derive(Default, Serialize)]
pub struct MonthSummary {
    pub received_zec: f64,
    pub sent_zec: f64,
    pub fees_zec: f64,
    pub net_zec: f64,
    pub tx_count: u32,
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn row_to_entry(r: &HistoryRow) -> LedgerEntry {
    let date = r
        .time
        .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string());
    let kind = if r.is_shielding {
        "shielding"
    } else if r.balance_delta >= 0 {
        "received"
    } else {
        "sent"
    }
    .to_string();
    LedgerEntry {
        txid: hex_encode(&r.txid),
        height: r.height,
        date,
        amount_zec: zats_to_zec(r.balance_delta),
        fee_zec: zats_to_zec(r.fee.unwrap_or(0)),
        kind,
    }
}

fn month_key(r: &HistoryRow) -> String {
    r.time
        .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
        .map(|dt| dt.format("%Y-%m").to_string())
        .unwrap_or_else(|| "pending".to_string())
}

pub fn generate_report(
    data_dir: &Path,
    out_base: &str,
    network: Network,
    passphrase: &str,
) -> Result<()> {
    let rows = read_history(data_dir, network, passphrase)?;

    if rows.is_empty() {
        println!();
        println!("  No transactions to report. (Empty or newly-synced wallet.)");
        return Ok(());
    }

    let mut months: BTreeMap<String, MonthSummary> = BTreeMap::new();
    for r in &rows {
        let m = months.entry(month_key(r)).or_default();
        let amt = zats_to_zec(r.balance_delta);
        if amt >= 0.0 {
            m.received_zec += amt;
        } else {
            m.sent_zec += -amt;
        }
        m.fees_zec += zats_to_zec(r.fee.unwrap_or(0));
        m.net_zec += amt;
        m.tx_count += 1;
    }

    println!();
    println!("  Monthly summary");
    println!("  {:-<70}", "");
    println!(
        "  {:<9}  {:>14}  {:>14}  {:>12}  {:>6}",
        "Month", "Received", "Sent", "Net", "Txs"
    );
    println!("  {:-<70}", "");
    for (month, s) in &months {
        println!(
            "  {:<9}  {:>14.8}  {:>14.8}  {:>12.8}  {:>6}",
            month, s.received_zec, s.sent_zec, s.net_zec, s.tx_count
        );
    }
    println!("  {:-<70}", "");

    let entries: Vec<LedgerEntry> = rows.iter().map(row_to_entry).collect();

    let json_path = format!("{out_base}.json");
    let json = serde_json::to_string_pretty(&entries).context("failed to serialize JSON")?;
    std::fs::write(&json_path, json).with_context(|| format!("failed to write {json_path}"))?;

    let csv_path = format!("{out_base}.csv");
    let mut wtr = csv::Writer::from_path(&csv_path)
        .with_context(|| format!("failed to create {csv_path}"))?;
    for e in &entries {
        wtr.serialize(e).context("failed to write CSV row")?;
    }
    wtr.flush().context("failed to flush CSV")?;

    println!();
    println!("  Full ledger written to:");
    println!("    {csv_path}");
    println!("    {json_path}");
    Ok(())
}
