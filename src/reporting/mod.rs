use crate::core::{Transaction, NetworkStats};
use crate::accounting::{generate_summary, generate_ledger};
use crate::payments::{build_payment_log, get_stats};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::json;

pub struct ReportFilter {
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub tx_type: Option<String>,
    pub min_amount: Option<f64>,
}

impl ReportFilter {
    pub fn new() -> Self {
        ReportFilter { from: None, to: None, tx_type: None, min_amount: None }
    }
}

pub fn filter_transactions<'a>(txs: &'a [Transaction], filter: &ReportFilter) -> Vec<&'a Transaction> {
    txs.iter().filter(|tx| {
        if let Some(from) = filter.from { if tx.timestamp < from { return false; } }
        if let Some(to) = filter.to { if tx.timestamp > to { return false; } }
        if let Some(ref t) = filter.tx_type {
            if tx.tx_type.to_string() != *t { return false; }
        }
        if let Some(min) = filter.min_amount { if tx.amount_zec < min { return false; } }
        true
    }).collect()
}

pub fn generate_full_report(txs: &[Transaction], path: &str, format: &str) -> Result<()> {
    let summary = generate_summary(txs);
    let ledger = generate_ledger(txs);
    let payments = build_payment_log(txs);
    let payment_stats = get_stats(&payments);
    let net_stats = NetworkStats::from_transactions(txs, 2_800_000);

    match format {
        "json" => {
            let report = json!({
                "generated_at": Utc::now(),
                "network": net_stats,
                "accounting": {
                    "summary": summary,
                    "ledger": ledger,
                },
                "payments": {
                    "stats": payment_stats,
                    "log": payments,
                }
            });
            std::fs::write(path, serde_json::to_string_pretty(&report)?)?;
        }
        "csv" => {
            let mut wtr = csv::Writer::from_path(path)?;
            wtr.write_record(["id","txid","date","type","debit_zec","credit_zec","fee_zec","balance_zec","debit_usd","credit_usd","description"])?;
            for entry in &ledger {
                wtr.write_record([
                    "",
                    &entry.txid,
                    &entry.date.format("%Y-%m-%d %H:%M").to_string(),
                    &entry.tx_type,
                    &format!("{:.8}", entry.debit_zec),
                    &format!("{:.8}", entry.credit_zec),
                    &format!("{:.8}", entry.fee_zec),
                    &format!("{:.8}", entry.balance_zec),
                    &format!("{:.2}", entry.debit_usd),
                    &format!("{:.2}", entry.credit_usd),
                    &entry.description,
                ])?;
            }
            wtr.flush()?;
        }
        _ => anyhow::bail!("Use csv or json for full reports"),
    }

    println!("\n Full report saved: {}", path);
    println!("  Transactions    : {}", summary.transaction_count);
    println!("  Total Received  : {:.4} ZEC (${:.2})", summary.total_received_zec, summary.total_received_usd);
    println!("  Total Sent      : {:.4} ZEC (${:.2})", summary.total_sent_zec, summary.total_sent_usd);
    println!("  Net Position    : {:.4} ZEC", summary.net_position_zec);
    println!("  Confirmed Pmts  : {}", payment_stats.confirmed);
    println!("  Pending Pmts    : {}", payment_stats.pending);
    Ok(())
}
