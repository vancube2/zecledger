use crate::core::Transaction;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PaymentStatus {
    Confirmed,
    Pending,
    Failed,
}

impl std::fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PaymentStatus::Confirmed => write!(f, "confirmed"),
            PaymentStatus::Pending   => write!(f, "pending"),
            PaymentStatus::Failed    => write!(f, "failed"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Payment {
    pub id: String,
    pub txid: String,
    pub timestamp: DateTime<Utc>,
    pub amount_zec: f64,
    pub amount_usd: f64,
    pub fee_zec: f64,
    pub status: PaymentStatus,
    pub tx_type: String,
    pub block_height: u64,
    pub confirmations: u64,
    pub memo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentStats {
    pub total_payments: u64,
    pub confirmed: u64,
    pub pending: u64,
    pub total_volume_zec: f64,
    pub total_volume_usd: f64,
    pub total_fees_zec: f64,
    pub largest_payment_zec: f64,
    pub smallest_payment_zec: f64,
    pub avg_payment_zec: f64,
}

pub fn build_payment_log(txs: &[Transaction]) -> Vec<Payment> {
    let current_block = 2_800_100_u64;
    txs.iter().enumerate().map(|(i, tx)| {
        let confirmations = current_block.saturating_sub(tx.block_height);
        let status = if confirmations >= 10 {
            PaymentStatus::Confirmed
        } else if confirmations > 0 {
            PaymentStatus::Pending
        } else {
            PaymentStatus::Pending
        };
        Payment {
            id: format!("PAY-{:04}", i + 1),
            txid: tx.txid[..12].to_string() + "...",
            timestamp: tx.timestamp,
            amount_zec: tx.amount_zec,
            amount_usd: tx.amount_usd.unwrap_or(0.0),
            fee_zec: tx.fee_zec,
            status,
            tx_type: tx.tx_type.to_string(),
            block_height: tx.block_height,
            confirmations,
            memo: tx.memo.clone(),
        }
    }).collect()
}

pub fn get_stats(payments: &[Payment]) -> PaymentStats {
    let total = payments.len() as u64;
    let confirmed = payments.iter().filter(|p| matches!(p.status, PaymentStatus::Confirmed)).count() as u64;
    let pending = payments.iter().filter(|p| matches!(p.status, PaymentStatus::Pending)).count() as u64;
    let total_vol: f64 = payments.iter().map(|p| p.amount_zec).sum();
    let total_usd: f64 = payments.iter().map(|p| p.amount_usd).sum();
    let total_fees: f64 = payments.iter().map(|p| p.fee_zec).sum();
    let largest = payments.iter().map(|p| p.amount_zec).fold(0.0_f64, f64::max);
    let smallest = payments.iter().map(|p| p.amount_zec).fold(f64::MAX, f64::min);

    PaymentStats {
        total_payments: total,
        confirmed,
        pending,
        total_volume_zec: total_vol,
        total_volume_usd: total_usd,
        total_fees_zec: total_fees,
        largest_payment_zec: largest,
        smallest_payment_zec: if smallest == f64::MAX { 0.0 } else { smallest },
        avg_payment_zec: if total > 0 { total_vol / total as f64 } else { 0.0 },
    }
}

pub fn print_payment_log(payments: &[Payment], limit: usize) {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║                  ZecLedger Payment Log                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!("{:<10} {:<15} {:<12} {:<10} {:<12} {:<10}",
        "ID", "TXID", "TYPE", "ZEC", "USD", "STATUS");
    println!("{}", "─".repeat(70));
    for payment in payments.iter().take(limit) {
        println!("{:<10} {:<15} {:<12} {:<10.4} {:<12.2} {:<10}",
            payment.id,
            payment.txid,
            payment.tx_type,
            payment.amount_zec,
            payment.amount_usd,
            payment.status,
        );
    }
    println!("{}", "─".repeat(70));
}

pub fn print_pending(payments: &[Payment]) {
    let pending: Vec<&Payment> = payments.iter()
        .filter(|p| matches!(p.status, PaymentStatus::Pending))
        .collect();
    println!("\n╔══════════════════════════════════════════════╗");
    println!("║         Pending Payments ({:>3})               ║", pending.len());
    println!("╚══════════════════════════════════════════════╝");
    if pending.is_empty() {
        println!("  No pending payments.");
        return;
    }
    for p in &pending {
        println!("  {} | {:.4} ZEC (${:.2}) | {} confirmations | {}",
            p.id, p.amount_zec, p.amount_usd, p.confirmations, p.tx_type);
    }
    println!("══════════════════════════════════════════════\n");
}

pub fn print_stats(stats: &PaymentStats) {
    println!("\n╔══════════════════════════════════════════════╗");
    println!("║          Payment Management Summary          ║");
    println!("╚══════════════════════════════════════════════╝");
    println!("  Total Payments  : {}", stats.total_payments);
    println!("  Confirmed       : {}", stats.confirmed);
    println!("  Pending         : {}", stats.pending);
    println!("──────────────────────────────────────────────");
    println!("  Total Volume    : {:.4} ZEC (${:.2})", stats.total_volume_zec, stats.total_volume_usd);
    println!("  Total Fees      : {:.6} ZEC", stats.total_fees_zec);
    println!("  Largest Payment : {:.4} ZEC", stats.largest_payment_zec);
    println!("  Smallest Payment: {:.4} ZEC", stats.smallest_payment_zec);
    println!("  Avg Payment     : {:.4} ZEC", stats.avg_payment_zec);
    println!("══════════════════════════════════════════════\n");
}
