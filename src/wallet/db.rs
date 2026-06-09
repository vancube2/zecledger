// src/wallet/db.rs
//
// Step 3a: wallet database initialization only.
// Creates and initializes the WalletDb on disk. No account, no network yet.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use rand::rngs::OsRng;
use zcash_client_sqlite::util::SystemClock;
use zcash_client_sqlite::WalletDb;
use zcash_protocol::consensus::MainNetwork;

/// Where the wallet database lives, under the configured data dir.
pub fn wallet_db_path(data_dir: &Path) -> PathBuf {
    data_dir.join("wallet.sqlite")
}

/// Open (creating if needed) and initialize the wallet database.
pub fn open_and_init(data_dir: &Path) -> Result<()> {
    fs::create_dir_all(data_dir)
        .with_context(|| format!("could not create data dir {}", data_dir.display()))?;

    let db_path = wallet_db_path(data_dir);
    println!("  Wallet database: {}", db_path.display());

    let mut db = WalletDb::for_path(&db_path, MainNetwork, SystemClock, OsRng)
        .context("failed to open wallet database")?;

    zcash_client_sqlite::wallet::init::init_wallet_db(&mut db, None)
        .context("failed to initialize wallet database schema")?;

    println!("  Wallet database initialized.");
    Ok(())
}
