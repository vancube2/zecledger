// src/wallet/history.rs
//
// Step 2a: read transaction history from the wallet database's v_transactions
// view. All SQL lives here, isolated, so a schema change touches one file.

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

use super::db::wallet_db_path;
use zcash_protocol::consensus::Network;

/// One row of transaction history, ready to display.
pub struct HistoryRow {
    pub height: Option<i64>,
    pub time: Option<i64>,
    pub balance_delta: i64,
    pub fee: Option<i64>,
    pub is_shielding: bool,
    pub txid: Vec<u8>,
    pub memo: Option<String>,
}

fn zats_to_zec(z: i64) -> f64 {
    z as f64 / 1e8
}

/// Decode a Zcash memo BLOB into readable text following the memo spec.
/// Returns None for the canonical "no memo" form or anything not human text.
fn decode_memo(blob: &[u8]) -> Option<String> {
    if blob.is_empty() {
        return None;
    }
    // First byte 0xF6 = "no memo"; 0xF5 = reserved/menu. 0x00..=0xF4 = UTF-8 text.
    match blob[0] {
        0xF6 | 0xF5 => return None,
        b if b >= 0xF7 => return Some("(non-text memo)".to_string()),
        _ => {}
    }
    // Strip trailing zero padding, then decode as UTF-8.
    let end = blob
        .iter()
        .rposition(|&b| b != 0)
        .map(|i| i + 1)
        .unwrap_or(0);
    let trimmed = &blob[..end];
    match std::str::from_utf8(trimmed) {
        Ok(text) => {
            let t = text.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        Err(_) => Some("(non-text memo)".to_string()),
    }
}

/// Read memos keyed by txid (transaction hash bytes).
/// Joins v_received_outputs to transactions; concatenates if a tx has several memos.
fn read_memos(conn: &rusqlite::Connection) -> Result<HashMap<Vec<u8>, String>> {
    let mut stmt = conn
        .prepare(
            "SELECT t.txid, ro.memo \
             FROM v_received_outputs ro \
             JOIN transactions t ON t.id_tx = ro.transaction_id \
             WHERE ro.memo IS NOT NULL",
        )
        .context("failed to prepare memo query")?;

    let mut map: HashMap<Vec<u8>, String> = HashMap::new();
    let rows = stmt
        .query_map([], |row| {
            let txid: Vec<u8> = row.get::<_, Option<Vec<u8>>>(0)?.unwrap_or_default();
            let memo: Vec<u8> = row.get::<_, Option<Vec<u8>>>(1)?.unwrap_or_default();
            Ok((txid, memo))
        })
        .context("failed to run memo query")?;

    for r in rows {
        let (txid, memo_blob) = r.context("failed to read memo row")?;
        if txid.is_empty() {
            continue;
        }
        if let Some(text) = decode_memo(&memo_blob) {
            map.entry(txid)
                .and_modify(|existing| {
                    existing.push_str(" | ");
                    existing.push_str(&text);
                })
                .or_insert(text);
        }
    }
    Ok(map)
}

/// Read all transactions for the wallet, most recent first.
pub fn read_history(data_dir: &Path, network: Network) -> Result<Vec<HistoryRow>> {
    let db_path = wallet_db_path(data_dir, network);
    let conn = rusqlite::Connection::open(&db_path)
        .context("could not open wallet database for history")?;

    let mut stmt = conn
        .prepare(
            "SELECT mined_height, block_time, account_balance_delta, \
                    fee_paid, is_shielding, txid \
             FROM v_transactions \
             ORDER BY mined_height DESC NULLS FIRST, tx_index DESC",
        )
        .context("failed to prepare history query")?;

    let rows = stmt
        .query_map([], |row| {
            Ok(HistoryRow {
                height: row.get(0)?,
                time: row.get(1)?,
                balance_delta: row.get::<_, Option<i64>>(2)?.unwrap_or(0),
                fee: row.get(3)?,
                is_shielding: row.get::<_, Option<i64>>(4)?.unwrap_or(0) != 0,
                txid: row.get::<_, Option<Vec<u8>>>(5)?.unwrap_or_default(),
                memo: None,
            })
        })
        .context("failed to run history query")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read history rows")?;

    // Attach memos (joined from v_received_outputs by txid).
    let memos = read_memos(&conn)?;
    let mut rows = rows;
    for row in rows.iter_mut() {
        if let Some(text) = memos.get(&row.txid) {
            row.memo = Some(text.clone());
        }
    }

    Ok(rows)
}

/// Print transaction history as a readable table.
pub fn print_history(rows: &[HistoryRow]) {
    if rows.is_empty() {
        println!();
        println!("  No transactions found for this wallet.");
        println!("  (An empty or newly-synced wallet will show nothing here.)");
        return;
    }
    println!();
    println!("  Transaction history ({} transactions)", rows.len());
    println!("  {:-<64}", "");
    println!(
        "  {:<12}  {:<19}  {:>16}  {:>10}",
        "Height", "Date", "Amount (ZEC)", "Type"
    );
    println!("  {:-<64}", "");
    for r in rows {
        let height = r
            .height
            .map(|h| h.to_string())
            .unwrap_or_else(|| "pending".to_string());
        let date = r
            .time
            .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "-".to_string());
        let amount = zats_to_zec(r.balance_delta);
        let kind = if r.is_shielding {
            "shielding"
        } else if r.balance_delta >= 0 {
            "received"
        } else {
            "sent"
        };
        println!("  {height:<12}  {date:<19}  {amount:>16.8}  {kind:>10}");
        if let Some(memo) = &r.memo {
            println!("                memo: {memo}");
        }
    }
    println!("  {:-<64}", "");
}
