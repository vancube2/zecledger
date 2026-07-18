// src/wallet/report.rs
//
// Step 2b: accounting reports. Monthly summary on screen, full ledger to
// files. Reads the same v_transactions view as history.
//
// The user chooses a format from the interactive menu; the plain `wallet-report`
// command keeps its old behaviour of writing both CSV and JSON. After writing,
// we tell the user exactly where the file landed, including a Windows-openable
// path when running the Linux build under WSL, because a path like
// /root/zecledger/report.csv is useless to someone who lives in Explorer.

use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::history::{read_history, HistoryRow};
use zcash_protocol::consensus::Network;

fn zats_to_zec(z: i64) -> f64 {
    z as f64 / 1e8
}

/// Which files a report should produce.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportFormat {
    Csv,
    Json,
    Both,
    Markdown,
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

fn write_json(out_base: &str, entries: &[LedgerEntry]) -> Result<String> {
    let json_path = format!("{out_base}.json");
    let json = serde_json::to_string_pretty(entries).context("failed to serialize JSON")?;
    std::fs::write(&json_path, json).with_context(|| format!("failed to write {json_path}"))?;
    Ok(json_path)
}

fn write_csv(out_base: &str, entries: &[LedgerEntry]) -> Result<String> {
    let csv_path = format!("{out_base}.csv");
    let mut wtr =
        csv::Writer::from_path(&csv_path).with_context(|| format!("failed to create {csv_path}"))?;
    for e in entries {
        wtr.serialize(e).context("failed to write CSV row")?;
    }
    wtr.flush().context("failed to flush CSV")?;
    Ok(csv_path)
}

fn write_markdown(
    out_base: &str,
    months: &BTreeMap<String, MonthSummary>,
    entries: &[LedgerEntry],
) -> Result<String> {
    let md_path = format!("{out_base}.md");
    let mut s = String::new();
    s.push_str("# ZecLedger accounting report\n\n");
    s.push_str(&format!(
        "Generated {}\n\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    s.push_str("## Monthly summary\n\n");
    s.push_str("| Month | Received (ZEC) | Sent (ZEC) | Net (ZEC) | Txs |\n");
    s.push_str("|---|---:|---:|---:|---:|\n");
    for (month, m) in months {
        s.push_str(&format!(
            "| {} | {:.8} | {:.8} | {:.8} | {} |\n",
            month, m.received_zec, m.sent_zec, m.net_zec, m.tx_count
        ));
    }

    s.push_str("\n## Full ledger\n\n");
    s.push_str("| Date | Type | Amount (ZEC) | Fee (ZEC) | Height | Txid |\n");
    s.push_str("|---|---|---:|---:|---:|---|\n");
    for e in entries {
        s.push_str(&format!(
            "| {} | {} | {:.8} | {:.8} | {} | {} |\n",
            e.date.clone().unwrap_or_else(|| "pending".to_string()),
            e.kind,
            e.amount_zec,
            e.fee_zec,
            e.height
                .map(|h| h.to_string())
                .unwrap_or_else(|| "-".to_string()),
            e.txid
        ));
    }

    std::fs::write(&md_path, s).with_context(|| format!("failed to write {md_path}"))?;
    Ok(md_path)
}

/// Turn a possibly-relative output name into a full path, without the ugly
/// `\\?\` verbatim prefix that canonicalize adds on Windows.
fn absolutize(p: &str) -> PathBuf {
    let pb = Path::new(p);
    if pb.is_absolute() {
        pb.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|d| d.join(pb))
            .unwrap_or_else(|_| pb.to_path_buf())
    }
}

/// When running the Linux build under WSL, give the path in a form Windows
/// Explorer can open. Returns None on a native OS, where the plain path is
/// already the right one.
fn windows_hint(abs: &Path) -> Option<String> {
    let distro = std::env::var("WSL_DISTRO_NAME").ok()?;
    let s = abs.to_string_lossy();
    if let Some(rest) = s.strip_prefix("/mnt/") {
        // /mnt/c/Users/DELL/x -> C:\Users\DELL\x
        let mut chars = rest.chars();
        let drive = chars.next()?.to_ascii_uppercase();
        let after = chars.as_str().trim_start_matches('/').replace('/', "\\");
        Some(format!("{drive}:\\{after}"))
    } else {
        // A Linux-filesystem path -> \\wsl.localhost\<distro>\...
        let after = s.trim_start_matches('/').replace('/', "\\");
        Some(format!("\\\\wsl.localhost\\{distro}\\{after}"))
    }
}

fn print_saved_guide(written: &[String]) {
    println!();
    if written.len() == 1 {
        println!("  Your report was saved. File:");
    } else {
        println!("  Your report was saved. Files:");
    }
    let mut folder_shown = false;
    for path in written {
        let abs = absolutize(path);
        println!("    {}", abs.display());
        if let Some(win) = windows_hint(&abs) {
            println!("      in Windows: {win}");
        }
        if !folder_shown {
            if let Some(parent) = abs.parent() {
                println!();
                println!("  To open it, go to this folder in your file manager:");
                println!("    {}", parent.display());
                if let Some(winparent) = windows_hint(parent) {
                    println!("    in Windows: {winparent}");
                }
                folder_shown = true;
            }
        }
    }
}

/// Format-aware report generator. Prints the monthly summary on screen, writes
/// the requested file(s), then tells the user where they are.
pub fn generate_report_with_format(
    data_dir: &Path,
    out_base: &str,
    network: Network,
    passphrase: &str,
    format: ReportFormat,
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

    let mut written: Vec<String> = Vec::new();
    match format {
        ReportFormat::Csv => written.push(write_csv(out_base, &entries)?),
        ReportFormat::Json => written.push(write_json(out_base, &entries)?),
        ReportFormat::Both => {
            written.push(write_csv(out_base, &entries)?);
            written.push(write_json(out_base, &entries)?);
        }
        ReportFormat::Markdown => written.push(write_markdown(out_base, &months, &entries)?),
    }

    print_saved_guide(&written);
    Ok(())
}

/// `zecledger report` - keeps its original behaviour: writes both CSV and JSON.
pub fn generate_report(
    data_dir: &Path,
    out_base: &str,
    network: Network,
    passphrase: &str,
) -> Result<()> {
    generate_report_with_format(data_dir, out_base, network, passphrase, ReportFormat::Both)
}
