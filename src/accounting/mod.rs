use crate::core::{Transaction, TxType};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountingSummary {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub opening_balance_zec: f64,
    pub total_received_zec: f64,
    pub total_sent_zec: f64,
    pub total_fees_zec: f64,
    pub net_position_zec: f64,
    pub total_received_usd: f64,
    pub total_sent_usd: f64,
    pub net_position_usd: f64,
    pub shielded_volume_zec: f64,
    pub transparent_volume_zec: f64,
    pub transaction_count: u64,
    pub avg_tx_fee_zec: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub date: DateTime<Utc>,
    pub txid: String,
    pub tx_type: String,
    pub debit_zec: f64,
    pub credit_zec: f64,
    pub fee_zec: f64,
    pub balance_zec: f64,
    pub debit_usd: f64,
    pub credit_usd: f64,
    pub description: String,
}

pub fn generate_summary(txs: &[Transaction]) -> AccountingSummary {
    let now = Utc::now();
    let period_start = txs.iter().map(|t| t.timestamp).min().unwrap_or(now);
    let period_end = txs.iter().map(|t| t.timestamp).max().unwrap_or(now);

    // Split into received (even index) and sent (odd index) for demo
    let received: Vec<&Transaction> = txs.iter().step_by(2).collect();
    let sent: Vec<&Transaction> = txs.iter().skip(1).step_by(3).collect();

    let total_received_zec: f64 = received.iter().map(|t| t.amount_zec).sum();
    let total_sent_zec: f64 = sent.iter().map(|t| t.amount_zec).sum();
    let total_fees_zec: f64 = txs.iter().map(|t| t.fee_zec).sum();
    let total_received_usd: f64 = received.iter().filter_map(|t| t.amount_usd).sum();
    let total_sent_usd: f64 = sent.iter().filter_map(|t| t.amount_usd).sum();

    let shielded_volume: f64 = txs.iter()
        .filter(|t| matches!(t.tx_type, TxType::Shielded))
        .map(|t| t.amount_zec).sum();
    let transparent_volume: f64 = txs.iter()
        .filter(|t| matches!(t.tx_type, TxType::Transparent))
        .map(|t| t.amount_zec).sum();

    let count = txs.len() as u64;

    AccountingSummary {
        period_start,
        period_end,
        opening_balance_zec: 0.0,
        total_received_zec,
        total_sent_zec,
        total_fees_zec,
        net_position_zec: total_received_zec - total_sent_zec - total_fees_zec,
        total_received_usd,
        total_sent_usd,
        net_position_usd: total_received_usd - total_sent_usd,
        shielded_volume_zec: shielded_volume,
        transparent_volume_zec: transparent_volume,
        transaction_count: count,
        avg_tx_fee_zec: if count > 0 { total_fees_zec / count as f64 } else { 0.0 },
    }
}

pub fn generate_ledger(txs: &[Transaction]) -> Vec<LedgerEntry> {
    let mut entries = Vec::new();
    let mut running_balance = 0.0_f64;

    for (i, tx) in txs.iter().enumerate() {
        let is_received = i % 2 == 0;
        let (debit, credit) = if is_received {
            (0.0, tx.amount_zec)
        } else {
            (tx.amount_zec, 0.0)
        };
        let debit_usd = if is_received { 0.0 } else { tx.amount_usd.unwrap_or(0.0) };
        let credit_usd = if is_received { tx.amount_usd.unwrap_or(0.0) } else { 0.0 };

        running_balance += credit - debit - tx.fee_zec;

        entries.push(LedgerEntry {
            date: tx.timestamp,
            txid: tx.txid[..8].to_string() + "...",
            tx_type: tx.tx_type.to_string(),
            debit_zec: debit,
            credit_zec: credit,
            fee_zec: tx.fee_zec,
            balance_zec: running_balance,
            debit_usd,
            credit_usd,
            description: format!("{} transaction on block {}", tx.tx_type, tx.block_height),
        });
    }
    entries
}

pub fn print_summary(summary: &AccountingSummary) {
    println!("\n╔══════════════════════════════════════════════╗");
    println!("║       ZecLedger Accounting Summary           ║");
    println!("╚══════════════════════════════════════════════╝");
    println!("  Period     : {} → {}",
        summary.period_start.format("%Y-%m-%d"),
        summary.period_end.format("%Y-%m-%d"));
    println!("  Transactions: {}", summary.transaction_count);
    println!("──────────────────────────────────────────────");
    println!("  INCOME");
    println!("  Total Received : {:.4} ZEC  (${:.2})", summary.total_received_zec, summary.total_received_usd);
    println!("──────────────────────────────────────────────");
    println!("  EXPENSES");
    println!("  Total Sent     : {:.4} ZEC  (${:.2})", summary.total_sent_zec, summary.total_sent_usd);
    println!("  Total Fees     : {:.6} ZEC", summary.total_fees_zec);
    println!("  Avg Fee/Tx     : {:.6} ZEC", summary.avg_tx_fee_zec);
    println!("──────────────────────────────────────────────");
    println!("  NET POSITION");
    println!("  Net ZEC        : {:.4} ZEC", summary.net_position_zec);
    println!("  Net USD        : ${:.2}", summary.net_position_usd);
    println!("──────────────────────────────────────────────");
    println!("  PRIVACY BREAKDOWN");
    println!("  Shielded Vol   : {:.4} ZEC", summary.shielded_volume_zec);
    println!("  Transparent Vol: {:.4} ZEC", summary.transparent_volume_zec);
    println!("══════════════════════════════════════════════\n");
}
