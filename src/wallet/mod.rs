// src/wallet/mod.rs
pub mod account;
pub mod cache;
pub mod costbasis;
pub mod db;
pub mod history;
pub mod passphrase;
pub mod privacy;
pub mod reconcile;
pub mod report;
pub mod request;
//
// The local, private side of ZecLedger: shielded accounting from a viewing key.
// Read-only by design. This module never holds or handles a spending key.

use anyhow::{anyhow, Context, Result};
use std::io::{self, Write};

use zcash_keys::keys::UnifiedFullViewingKey;
use zcash_protocol::consensus::Network;

/// Everything we hold for one session. The key is imported into the wallet db on sync.
pub struct WalletSession {
    pub ufvk: UnifiedFullViewingKey,
    pub birthday: u32,
}

/// The security reminder shown every session before a viewing key is requested.
pub fn print_key_safety_reminder() {
    println!();
    println!("  ZecLedger is read-only. It uses a viewing key, never a spending key,");
    println!("  so it cannot move your funds.");
    println!("  Your key is never sent to any server. It stays on this machine.");
    println!("  It is stored in your local wallet database so ZecLedger can scan the");
    println!("  chain for your notes. Anyone who can read that file can see your");
    println!("  transaction history, so keep it protected.");
    println!();
}

/// Prompt for UFVK and birthday, validate the key, return an in-memory session.
pub fn prompt_for_session(network: Network) -> Result<WalletSession> {
    print_key_safety_reminder();

    let key_prefix = match network {
        Network::TestNetwork => "uviewtest",
        _ => "uview",
    };
    print!("Paste your Unified Full Viewing Key (starts with '{key_prefix}'): ");
    io::stdout().flush().ok();
    let mut ufvk_str = String::new();
    io::stdin()
        .read_line(&mut ufvk_str)
        .context("failed to read viewing key")?;
    let ufvk_str = ufvk_str.trim().to_string();
    if ufvk_str.is_empty() {
        return Err(anyhow!("no viewing key entered"));
    }

    let ufvk = UnifiedFullViewingKey::decode(&network, &ufvk_str)
        .map_err(|e| anyhow!("that does not look like a valid Unified Full Viewing Key: {e}"))?;
    println!("  Viewing key looks valid.");

    print!("Enter your wallet birthday block height (e.g. 2700000): ");
    io::stdout().flush().ok();
    let mut bday = String::new();
    io::stdin()
        .read_line(&mut bday)
        .context("failed to read birthday height")?;
    let birthday: u32 = bday
        .trim()
        .parse()
        .context("birthday must be a whole number block height")?;

    println!("  Viewing key accepted for this session.");
    println!();
    Ok(WalletSession { ufvk, birthday })
}

pub async fn show_balance(network: Network) -> Result<()> {
    use zcash_client_backend::data_api::wallet::ConfirmationsPolicy;
    use zcash_client_backend::data_api::WalletRead;

    let config = crate::core::config::load()?;
    let pass = passphrase::prompt_existing()?;
    let db = db::open_wallet_db(&config.data_dir, network, &pass)?;

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
            for b in balances.values() {
                let sapling = zec(b.sapling_balance().total());
                let orchard = zec(b.orchard_balance().total());
                let ironwood = zec(b.ironwood_balance().total());
                let transparent = zec(b.unshielded_balance().total());
                let total = zec(b.total());
                println!("  Sapling:      {sapling:>14.8} ZEC", sapling = sapling);
                println!("  Orchard:      {orchard:>14.8} ZEC", orchard = orchard);
                println!("  Ironwood:     {ironwood:>14.8} ZEC", ironwood = ironwood);
                println!(
                    "  Transparent:  {transparent:>14.8} ZEC",
                    transparent = transparent
                );
                println!("  ----------------");
                println!("  Total:        {total:>14.8} ZEC", total = total);
            }
            println!();
            println!("  Scanned to height {}.", summary.fully_scanned_height());
        }
    }
    Ok(())
}

pub async fn sync(network: Network, endpoint: String) -> Result<()> {
    let config = crate::core::config::load()?;
    let db_path = db::wallet_db_path(&config.data_dir, network);

    let migrating = db::is_plaintext(&db_path);
    let is_new = !db_path.exists();

    // An older database from before encryption. Say so plainly before asking.
    if migrating {
        println!();
        println!("  This wallet database was created before ZecLedger encrypted its");
        println!("  data, so your viewing key and history are currently sitting in it");
        println!("  unencrypted. ZecLedger will encrypt it now.");
    }

    let pass = if migrating || is_new {
        passphrase::prompt_new()?
    } else {
        passphrase::prompt_existing()?
    };

    if migrating {
        let backup = db::encrypt_in_place(&db_path, &pass)?;
        println!();
        println!("  Encrypted. The old unencrypted file is still on disk at:");
        println!("    {}", backup.display());
        println!("  Delete it once you are happy, because it still holds your key.");
        println!();
    }

    // Always run this, on every path. It applies any pending schema migrations,
    // which is how a database created by an older ZecLedger gains the tables a
    // newer network upgrade needs. Skipping it on the migration path was a bug.
    db::open_and_init(&config.data_dir, network, &pass)?;

    // The key only needs pasting once. After that it is already in the database.
    if db::has_account(&config.data_dir, network, &pass)? {
        println!("  Using the viewing key already in your wallet database.");
    } else {
        let session = prompt_for_session(network)?;
        println!(
            "Got a valid viewing key, birthday height {}.",
            session.birthday
        );
        account::import_view_only(
            &config.data_dir,
            &endpoint,
            &session.ufvk,
            session.birthday as u64,
            network,
            &pass,
        )
        .await?;
    }

    finish_sync(&config.data_dir, &endpoint, network, &pass).await
}

async fn finish_sync(
    data_dir: &std::path::Path,
    endpoint: &str,
    network: Network,
    pass: &str,
) -> Result<()> {
    account::sync_blocks(data_dir, endpoint, network, pass).await?;
    println!("Wallet synced.");
    Ok(())
}

/// `zecledger history` - show transaction history from the synced wallet.
pub async fn show_history(network: Network) -> Result<()> {
    let config = crate::core::config::load()?;
    let pass = passphrase::prompt_existing()?;
    let rows = history::read_history(&config.data_dir, network, &pass)?;
    history::print_history(&rows);
    Ok(())
}

/// `zecledger report` - monthly summary on screen, full ledger to CSV + JSON.
pub async fn generate_report(output: Option<String>, network: Network) -> Result<()> {
    let config = crate::core::config::load()?;
    let out_base = output.unwrap_or_else(|| {
        format!(
            "zecledger_ledger_{}",
            chrono::Utc::now().format("%Y%m%d_%H%M%S")
        )
    });
    let pass = passphrase::prompt_existing()?;
    report::generate_report(&config.data_dir, &out_base, network, &pass)?;
    Ok(())
}

/// `zecledger wallet-ask` - answer a question about YOUR wallet.
/// Reads only local data, shows exactly what would be sent, and requires
/// explicit confirmation before anything leaves the machine.
pub async fn wallet_ask(question: &str, network: Network) -> Result<()> {
    use std::io::{self, Write};

    let config = crate::core::config::load()?;
    let pass = passphrase::prompt_existing()?;
    let rows = history::read_history(&config.data_dir, network, &pass)?;

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
        if lines.is_empty() {
            "  (none)\n".to_string()
        } else {
            lines
        },
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
            "model": "claude-sonnet-4-6",
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

/// Record an expected payment (Phase 3a).
pub fn expect_payment(amount: f64, reference: &str, from: &str, network: Network) -> Result<()> {
    let config = crate::core::config::load()?;
    reconcile::add_expected(&config.data_dir, network, amount, reference, from)
}

/// Reconcile expected payments against received history (Phase 3a).
pub fn reconcile_payments(network: Network) -> Result<()> {
    let config = crate::core::config::load()?;
    let pass = passphrase::prompt_existing()?;
    reconcile::reconcile(&config.data_dir, network, &pass)
}

/// List expected payments (Phase 3a).
pub fn list_expected(network: Network) -> Result<()> {
    let config = crate::core::config::load()?;
    reconcile::list_expected(&config.data_dir, network)
}

/// Generate a ZIP-321 payment request URI (Phase 3b).
pub fn make_payment_request(
    address: &str,
    amount: f64,
    memo: Option<&str>,
    label: Option<&str>,
    message: Option<&str>,
) -> Result<()> {
    request::make_request(address, amount, memo, label, message)
}

/// Generate a cost-basis / gain-loss report (creative feature).
pub async fn cost_basis_report(method: &str, fetch: bool, network: Network) -> Result<()> {
    let config = crate::core::config::load()?;
    let m: costbasis::Method = method.parse()?;
    let pass = passphrase::prompt_existing()?;
    costbasis::report(&config.data_dir, network, m, fetch, &pass).await
}

/// Generate a privacy-hygiene report (creative feature).
pub fn privacy_report(network: Network) -> Result<()> {
    let config = crate::core::config::load()?;
    let pass = passphrase::prompt_existing()?;
    privacy::report(&config.data_dir, network, &pass)
}
