// src/wallet/privacy.rs
//
// Privacy-hygiene report. Analyzes the user's own transaction outputs to flag
// patterns that can weaken on-chain privacy, and gives a practical tip for each.
// All analysis is local and read-only. It reads pool/value data directly from
// v_received_outputs (pool: 0 = transparent, 2 = Sapling, 3 = Orchard) so it is
// self-contained and does not disturb the history pipeline.
//
// What it checks (only things the wallet data can actually show):
//   - Pool usage: how much activity is shielded vs publicly transparent
//   - Transparent exposure: transparent outputs are visible on-chain
//   - Round amounts: exact round values are easier to fingerprint/correlate
// It deliberately does not claim to detect address reuse or timing correlation,
// since the available data cannot establish those reliably.

use anyhow::{Context, Result};
use std::path::Path;
use zcash_protocol::consensus::Network;

use super::db::wallet_db_path;

struct OutputRow {
    pool: i64,
    value_zats: i64,
}

fn zats_to_zec(z: i64) -> f64 {
    z as f64 / 1e8
}

/// Is an amount suspiciously round (whole or half ZEC)? A soft signal only.
fn is_round_amount(zec: f64) -> bool {
    if zec <= 0.0 {
        return false;
    }
    let scaled = zec * 2.0;
    (scaled - scaled.round()).abs() < 1e-8 && zec >= 1.0
}

/// Run the privacy-hygiene report.
pub fn report(data_dir: &Path, network: Network) -> Result<()> {
    let db_path = wallet_db_path(data_dir, network);
    let conn = rusqlite::Connection::open(&db_path)
        .context("could not open wallet database for privacy report")?;

    let mut stmt = conn
        .prepare("SELECT pool, value FROM v_received_outputs")
        .context("failed to prepare privacy query")?;

    let outputs = stmt
        .query_map([], |row| {
            Ok(OutputRow {
                pool: row.get::<_, Option<i64>>(0)?.unwrap_or(-1),
                value_zats: row.get::<_, Option<i64>>(1)?.unwrap_or(0),
            })
        })
        .context("failed to run privacy query")?
        .collect::<rusqlite::Result<Vec<_>>>()
        .context("failed to read privacy rows")?;

    println!();
    println!("  Privacy hygiene report");
    println!("  {:-<70}", "");

    if outputs.is_empty() {
        println!("  No outputs found yet. Sync a wallet with activity to analyze.");
        println!("  {:-<70}", "");
        return Ok(());
    }

    // Tally by pool.
    let total = outputs.len();
    let mut transparent_count = 0usize;
    let mut sapling_count = 0usize;
    let mut orchard_count = 0usize;
    let mut transparent_value = 0i64;
    let mut round_count = 0usize;

    for o in &outputs {
        match o.pool {
            0 => {
                transparent_count += 1;
                transparent_value += o.value_zats;
            }
            2 => sapling_count += 1,
            3 => orchard_count += 1,
            _ => {}
        }
        if is_round_amount(zats_to_zec(o.value_zats)) {
            round_count += 1;
        }
    }

    let shielded_count = sapling_count + orchard_count;
    let shielded_pct = (shielded_count as f64 / total as f64) * 100.0;

    // Pool breakdown.
    println!("  Outputs analyzed : {}", total);
    println!(
        "  Shielded         : {} ({:.0}%)  [{} Orchard, {} Sapling]",
        shielded_count, shielded_pct, orchard_count, sapling_count
    );
    println!("  Transparent      : {}", transparent_count);
    println!("  {:-<70}", "");

    // Findings + tips.
    let mut findings = 0;

    if transparent_count > 0 {
        findings += 1;
        println!(
            "  [exposure]  {} transparent output(s), totaling {:.8} ZEC, are publicly visible on-chain.",
            transparent_count,
            zats_to_zec(transparent_value)
        );
        println!("              Tip: receive into a shielded (Orchard) address to keep amounts private.");
    }

    if round_count > 0 {
        findings += 1;
        println!(
            "  [pattern]   {} output(s) are round amounts (e.g. 1, 5, 10 ZEC), which are easier to correlate.",
            round_count
        );
        println!("              Tip: where practical, avoid exact round amounts to reduce fingerprinting.");
    }

    if findings == 0 {
        println!("  No privacy concerns found in the available data.");
        if transparent_count == 0 {
            println!("  All activity is shielded, which is a strong privacy posture.");
        }
    }

    println!("  {:-<70}", "");
    println!("  Note: this checks only what your wallet data can show (pool usage and amounts).");
    println!("  It cannot see address reuse or timing patterns, so a clean report is not a guarantee.");
    Ok(())
}
