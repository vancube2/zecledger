// src/wallet/mod.rs
pub mod db;
//
// The local, private side of ZecLedger: shielded accounting from a viewing key.
// Read-only by design. This module never holds or handles a spending key.

use anyhow::{anyhow, Context, Result};
use std::io::{self, Write};

use zcash_keys::keys::UnifiedFullViewingKey;
use zcash_protocol::consensus::MainNetwork;

/// A per-pool shielded balance snapshot. Filled in for real at Step 4.
#[derive(Debug, Clone, Default)]
pub struct ShieldedBalance {
    pub sapling_zec: f64,
    pub orchard_zec: f64,
    pub transparent_zec: f64,
}

impl ShieldedBalance {
    pub fn total_zec(&self) -> f64 {
        self.sapling_zec + self.orchard_zec + self.transparent_zec
    }
}

/// Everything we hold in memory for one session. Never written to disk.
pub struct WalletSession {
    pub ufvk: UnifiedFullViewingKey,
    pub ufvk_str: String,
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
    Ok(WalletSession { ufvk, ufvk_str, birthday })
}

pub async fn show_balance() -> Result<()> {
    let session = prompt_for_session()?;
    println!("Got a valid viewing key, birthday height {}.", session.birthday);
    println!("Shielded balance is not implemented yet (Phase 1, Step 4).");
    Ok(())
}

pub async fn sync() -> Result<()> {
    let session = prompt_for_session()?;
    println!("Got a valid viewing key, birthday height {}.", session.birthday);
    let config = crate::core::config::load()?;
    db::open_and_init(&config.data_dir)?;
    println!("Step 3a done: wallet database ready.");
    Ok(())
}
