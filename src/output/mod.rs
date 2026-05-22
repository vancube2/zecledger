pub mod tui;
use crate::core::{NetworkStats, Transaction};
use anyhow::Result;
use chrono::Utc;

pub async fn generate(txs: &[Transaction], format: &str, output_path: Option<&str>) -> Result<()> {
    let stats = NetworkStats::from_transactions(txs, 2_800_000);
    let default_name = format!("zecledger_{}.{}", Utc::now().format("%Y%m%d_%H%M%S"), format);
    let path = output_path.unwrap_or(&default_name);
    match format {
        "csv"  => generate_csv(txs, path)?,
        "json" => generate_json(txs, &stats, path)?,
        _ => anyhow::bail!("Use csv or json"),
    }
    println!("\n Report saved: {}", path);
    println!("   Transactions : {}", stats.total_transactions);
    println!("   Shield rate  : {:.1}%", stats.shield_rate_pct);
    println!("   Volume       : {:.4} ZEC (${:.2})", stats.total_volume_zec, stats.total_volume_usd);
    Ok(())
}

fn generate_csv(txs: &[Transaction], path: &str) -> Result<()> {
    let mut wtr = csv::Writer::from_path(path)?;
    wtr.write_record(["txid","block","timestamp","type","amount_zec","amount_usd","fee_zec"])?;
    for tx in txs {
        wtr.write_record([
            &tx.txid,
            &tx.block_height.to_string(),
            &tx.timestamp.to_rfc3339(),
            &tx.tx_type.to_string(),
            &format!("{:.8}", tx.amount_zec),
            &tx.amount_usd.map(|u| format!("{:.2}", u)).unwrap_or_default(),
            &format!("{:.8}", tx.fee_zec),
        ])?;
    }
    wtr.flush()?;
    Ok(())
}

fn generate_json(txs: &[Transaction], stats: &NetworkStats, path: &str) -> Result<()> {
    let report = serde_json::json!({ "generated_at": Utc::now(), "summary": stats, "transactions": txs });
    std::fs::write(path, serde_json::to_string_pretty(&report)?)?;
    Ok(())
}
