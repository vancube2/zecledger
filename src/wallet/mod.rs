// src/wallet/mod.rs
pub mod db;
pub mod account;
pub mod cache;
pub mod history;
pub mod report;
//
// The local, private side of ZecLedger: shielded accounting from a viewing key.
// Read-only by design. This module never holds or handles a spending key.

use anyhow::{anyhow, Context, Result};
use std::io::{self, Write};

use zcash_keys::keys::UnifiedFullViewingKey;
use zcash_protocol::consensus::MainNetwork;


/// Everything we hold in memory for one session. Never written to disk.
pub struct WalletSession {
    pub ufvk: UnifiedFullViewingKey,
    pub birthday: u32,
}

/// The security reminder shown every session before a viewing key is requested.
pub fn print_key_safety_reminder() {
    println!();
    println!("  ZecLedger is read-only. It uses a viewing key, never a spending key.");
    println!("  Your viewing key is held in memory only, for this session.");
    println!("  It is never written to disk and never sent to any server.");
    println!("  When this program exits, the key is gone. You re-enter it next time.");
    println!();
}

/// Prompt for UFVK and birthday, validate the key, return an in-memory session.
pub fn prompt_for_session() -> Result<WalletSession> {
    print_key_safety_reminder();

    print!("Paste your Unified Full Viewing Key (starts with 'uview'): ");
    io::stdout().flush().ok();
    let mut ufvk_str = String::new();
    io::stdin().read_line(&mut ufvk_str).context("failed to read viewing key")?;
    let ufvk_str = ufvk_str.trim().to_string();
    if ufvk_str.is_empty() {
        return Err(anyhow!("no viewing key entered"));
    }

    let ufvk = UnifiedFullViewingKey::decode(&MainNetwork, &ufvk_str)
        .map_err(|e| anyhow!("that does not look like a valid Unified Full Viewing Key: {e}"))?;
    println!("  Viewing key looks valid.");

    print!("Enter your wallet birthday block height (e.g. 2700000): ");
    io::stdout().flush().ok();
    let mut bday = String::new();
    io::stdin().read_line(&mut bday).context("failed to read birthday height")?;
    let birthday: u32 = bday.trim().parse().context("birthday must be a whole number block height")?;

    println!("  Session ready. Key held in memory only.");
    println!();
    Ok(WalletSession { ufvk, birthday })
}

pub async fn show_balance() -> Result<()> {
    use rand::rngs::OsRng;
    use zcash_client_backend::data_api::WalletRead;
    use zcash_client_backend::data_api::wallet::ConfirmationsPolicy;
    use zcash_client_sqlite::util::SystemClock;
    use zcash_client_sqlite::WalletDb;
    use zcash_protocol::consensus::MainNetwork;

    let _session = prompt_for_session()?;
    let config = crate::core::config::load()?;
    let db_path = db::wallet_db_path(&config.data_dir);
    let db = WalletDb::for_path(&db_path, MainNetwork, SystemClock, OsRng)
        .map_err(|e| anyhow::anyhow!("could not open wallet database: {e}"))?;

    let summary = db
        .get_wallet_summary(ConfirmationsPolicy::MIN)
        .map_err(|e| anyhow::anyhow!("could not read wallet summary: {e}"))?;

    let zec = |z: zcash_protocol::value::Zatoshis| -> f64 { z.into_u64() as f64 / 1e8 };

    match summary {
        None => {
            println!();
            println!("  No wallet summary yet. Run `zecledger sync` first.");
        }
        Some(summary) => {
            let balances = summary.account_balances();
            if balances.is_empty() {
                println!();
                println!("  No accounts found. Run `zecledger sync` first.");
            }
            println!();
            println!("  Shielded balance");
            println!("  ----------------");
            for (_id, b) in balances.iter() {
                let sapling = zec(b.sapling_balance().total());
                let orchard = zec(b.orchard_balance().total());
                let transparent = zec(b.unshielded_balance().total());
                let total = zec(b.total());
                println!("  Sapling:      {sapling:>14.8} ZEC", sapling = sapling);
                println!("  Orchard:      {orchard:>14.8} ZEC", orchard = orchard);
                println!("  Transparent:  {transparent:>14.8} ZEC", transparent = transparent);
                println!("  ----------------");
                println!("  Total:        {total:>14.8} ZEC", total = total);
            }
            println!();
            println!("  Scanned to height {}.", summary.fully_scanned_height());
        }
    }
    Ok(())
}

pub async fn sync() -> Result<()> {
    let session = prompt_for_session()?;
    println!("Got a valid viewing key, birthday height {}.", session.birthday);
    let config = crate::core::config::load()?;
    db::open_and_init(&config.data_dir)?;
    let endpoint = if config.lightwalletd_url.starts_with("http") {
        config.lightwalletd_url.clone()
    } else {
        format!("https://{}", config.lightwalletd_url)
    };
    account::import_view_only(&config.data_dir, &endpoint, &session.ufvk, session.birthday as u64).await?;
    account::sync_blocks(&config.data_dir, &endpoint).await?;
    println!("Step 3 done: wallet synced.");
    Ok(())
}

/// `zecledger history` - show transaction history from the synced wallet.
pub async fn show_history() -> Result<()> {
    let config = crate::core::config::load()?;
    let rows = history::read_history(&config.data_dir)?;
    history::print_history(&rows);
    Ok(())
}

/// `zecledger report` - monthly summary on screen, full ledger to CSV + JSON.
pub async fn generate_report(output: Option<String>) -> Result<()> {
    let config = crate::core::config::load()?;
    let out_base = output.unwrap_or_else(|| {
        format!("zecledger_ledger_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"))
    });
    report::generate_report(&config.data_dir, &out_base)?;
    Ok(())
}

/// `zecledger wallet-ask` - answer a question about YOUR wallet.
/// Reads only local data, shows exactly what would be sent, and requires
/// explicit confirmation before anything leaves the machine.
pub async fn wallet_ask(question: &str) -> Result<()> {
    use std::io::{self, Write};

    let config = crate::core::config::load()?;
    let rows = history::read_history(&config.data_dir)?;

    let mut received = 0i64;
    let mut sent = 0i64;
    let mut lines = String::new();
    for r in &rows {
        if r.balance_delta >= 0 {
            received += r.balance_delta;
        } else {
            sent += -r.balance_delta;
        }
        let date = r
            .time
            .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
            .map(|dt| dt.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "pending".to_string());
        let kind = if r.is_shielding {
            "shielding"
        } else if r.balance_delta >= 0 {
            "received"
        } else {
            "sent"
        };
        lines.push_str(&format!(
            "  {date}  {:+.8} ZEC  {kind}\n",
            r.balance_delta as f64 / 1e8
        ));
    }

    let context = format!(
        "Wallet summary (local, no addresses or memos):\n\
         - Transactions: {}\n\
         - Total received: {:.8} ZEC\n\
         - Total sent: {:.8} ZEC\n\
         - Net: {:.8} ZEC\n\
         Transaction list (date, amount, type):\n{}",
        rows.len(),
        received as f64 / 1e8,
        sent as f64 / 1e8,
        (received - sent) as f64 / 1e8,
        if lines.is_empty() { "  (none)\n".to_string() } else { lines },
    );

    println!();
    println!("  The copilot needs to send this data to answer your question:");
    println!("  {:-<60}", "");
    for line in context.lines() {
        println!("  {line}");
    }
    println!("  {:-<60}", "");
    println!("  This goes to the Anthropic API. No addresses or memos are included.");
    print!("  Send this and get an answer? [y/N]: ");
    io::stdout().flush().ok();
    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm)?;
    if confirm.trim().to_lowercase() != "y" {
        println!("  Cancelled. Nothing was sent.");
        return Ok(());
    }

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("Set: export ANTHROPIC_API_KEY=sk-ant-..."))?;

    println!("  Sending...");
    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "system": "You are ZecLedger Copilot. Answer the user's question about THEIR OWN Zcash wallet using only the data provided. Be precise with numbers. Do not speculate beyond the data.",
            "messages": [{
                "role": "user",
                "content": format!("{}\n\nQuestion: {}", context, question)
            }]
        }))
        .send().await?;

    let status = response.status();
    let body: serde_json::Value = response.json().await?;
    if !status.is_success() {
        println!("  API Error: {}", body);
        return Ok(());
    }
    let answer = body["content"][0]["text"].as_str().unwrap_or("No response");
    println!();
    println!("  Answer:");
    println!("  {:-<60}", "");
    for line in answer.lines() {
        println!("  {line}");
    }
    println!("  {:-<60}", "");
    Ok(())
}
