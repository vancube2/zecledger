pub mod config;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub txid: String,
    pub block_height: u64,
    pub timestamp: DateTime<Utc>,
    pub tx_type: TxType,
    pub amount_zec: f64,
    pub amount_usd: Option<f64>,
    pub fee_zec: f64,
    pub memo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TxType { Shielded, Transparent, Mixed }

impl std::fmt::Display for TxType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TxType::Shielded => write!(f, "shielded"),
            TxType::Transparent => write!(f, "transparent"),
            TxType::Mixed => write!(f, "mixed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub total_transactions: u64,
    pub shielded_count: u64,
    pub transparent_count: u64,
    pub total_volume_zec: f64,
    pub total_volume_usd: f64,
    pub avg_tx_size_zec: f64,
    pub shield_rate_pct: f64,
    pub block_height: u64,
    pub generated_at: DateTime<Utc>,
}

impl NetworkStats {
    pub fn from_transactions(txs: &[Transaction], block_height: u64) -> Self {
        let total = txs.len() as u64;
        let shielded = txs.iter().filter(|t| matches!(t.tx_type, TxType::Shielded)).count() as u64;
        let transparent = txs.iter().filter(|t| matches!(t.tx_type, TxType::Transparent)).count() as u64;
        let total_vol: f64 = txs.iter().map(|t| t.amount_zec).sum();
        let total_usd: f64 = txs.iter().filter_map(|t| t.amount_usd).sum();
        let shield_rate = if total > 0 { (shielded as f64 / total as f64) * 100.0 } else { 0.0 };
        NetworkStats {
            total_transactions: total,
            shielded_count: shielded,
            transparent_count: transparent,
            total_volume_zec: total_vol,
            total_volume_usd: total_usd,
            avg_tx_size_zec: if total > 0 { total_vol / total as f64 } else { 0.0 },
            shield_rate_pct: shield_rate,
            block_height,
            generated_at: Utc::now(),
        }
    }
}
