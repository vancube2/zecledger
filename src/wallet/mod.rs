// src/wallet/mod.rs
//
// The local, private side of ZecLedger: shielded accounting from a viewing key.
// Read-only by design. This module never holds or handles a spending key.
//
// Build order:
//   Step 1 (now)  - scaffolding: commands exist, print "not yet implemented"
//   Step 2        - viewing-key input (memory only, with reminder)
//   Step 3        - sync engine (zcash_client_sqlite + backend sync) against zec.rocks
//   Step 4        - balance readout per pool (Sapling, Orchard, transparent)

use anyhow::Result;

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

/// The security reminder shown every session before a viewing key is requested.
pub fn print_key_safety_reminder() {
    println!();
    println!("  ZecLedger is read-only. It uses a viewing key, never a spending key.");
    println!("  Your viewing key is held in memory only, for this session.");
    println!("  It is never written to disk and never sent to any server.");
    println!("  When this program exits, the key is gone. You re-enter it next time.");
    println!();
}

/// `zecledger balance` - Step 4 will compute and print the real per-pool balance.
pub async fn show_balance() -> Result<()> {
    print_key_safety_reminder();
    println!("Shielded balance is not implemented yet (Phase 1, Step 4).");
    println!("Coming: Sapling + Orchard + transparent totals from your viewing key.");
    Ok(())
}

/// `zecledger sync` - Step 3 will run the light-client sync against zec.rocks.
pub async fn sync() -> Result<()> {
    print_key_safety_reminder();
    println!("Wallet sync is not implemented yet (Phase 1, Step 3).");
    println!("Coming: download compact blocks from zec.rocks and decrypt locally.");
    Ok(())
}
