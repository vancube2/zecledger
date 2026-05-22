use crate::core::{Transaction, TxType};
use anyhow::Result;
use chrono::{Duration, Utc};
use uuid::Uuid;

pub async fn fetch_transactions(blocks: u32) -> Result<Vec<Transaction>> {
    Ok(generate_mock_transactions(blocks))
}

pub fn generate_mock_transactions(blocks: u32) -> Vec<Transaction> {
    let total = (blocks * 3) as usize;
    let mut txs = Vec::with_capacity(total);
    let now = Utc::now();
    let zec_price = 28.50_f64;
    let types = [TxType::Shielded, TxType::Shielded, TxType::Transparent, TxType::Mixed];

    for i in 0..total {
        let tx_type = types[i % types.len()].clone();
        let amount = match &tx_type {
            TxType::Shielded    => 0.5 + (i as f64 % 10.0) * 0.8,
            TxType::Transparent => 1.0 + (i as f64 % 5.0)  * 2.5,
            TxType::Mixed       => 0.25 + (i as f64 % 8.0) * 0.6,
        };
        txs.push(Transaction {
            txid: Uuid::new_v4().to_string().replace("-", ""),
            block_height: 2_800_000 + (i as u64 / 3),
            timestamp: now - Duration::minutes(i as i64 * 2),
            amount_usd: Some(amount * zec_price),
            fee_zec: 0.0001,
            memo: if matches!(tx_type, TxType::Shielded) {
                Some("ZecLedger".to_string())
            } else { None },
            tx_type,
            amount_zec: amount,
        });
    }
    txs
}
