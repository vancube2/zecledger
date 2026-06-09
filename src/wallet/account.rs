// src/wallet/account.rs
//
// Step 3b: connect to lightwalletd, fetch the treestate at the birthday height,
// build an AccountBirthday, and import the UFVK as a view-only account.

use anyhow::{anyhow, Context, Result};
use std::path::Path;

use rand::rngs::OsRng;
use zcash_client_backend::data_api::{AccountBirthday, AccountPurpose, WalletWrite};
use zcash_client_backend::proto::service::{
    compact_tx_streamer_client::CompactTxStreamerClient, BlockId,
};
use zcash_client_sqlite::util::SystemClock;
use zcash_client_sqlite::WalletDb;
use zcash_keys::keys::UnifiedFullViewingKey;
use zcash_protocol::consensus::MainNetwork;

use super::db::wallet_db_path;

/// Connect to lightwalletd, build the birthday, import the UFVK as view-only.
pub async fn import_view_only(
    data_dir: &Path,
    endpoint: &str,
    ufvk: &UnifiedFullViewingKey,
    birthday_height: u64,
) -> Result<()> {
    // 1. Connect to lightwalletd (zec.rocks).
    println!("  Connecting to {endpoint} ...");
    let mut client = CompactTxStreamerClient::connect(endpoint.to_string())
        .await
        .with_context(|| format!("could not connect to {endpoint}"))?;
    println!("  Connected.");

    // 2. Fetch the treestate for the block just before the birthday height.
    let request_height = birthday_height.saturating_sub(1);
    let treestate = client
        .get_tree_state(BlockId { height: request_height, hash: vec![] })
        .await
        .context("failed to fetch treestate from lightwalletd")?
        .into_inner();
    println!("  Fetched treestate at height {request_height}.");

    // 3. Build the AccountBirthday from that treestate.
    let birthday = AccountBirthday::from_treestate(treestate, None)
        .map_err(|_| anyhow!("could not build account birthday from treestate"))?;

    // 4. Open the wallet DB and import the UFVK as a view-only account.
    let db_path = wallet_db_path(data_dir);
    let mut db = WalletDb::for_path(&db_path, MainNetwork, SystemClock, OsRng)
        .context("failed to open wallet database")?;

    db.import_account_ufvk("main", ufvk, &birthday, AccountPurpose::ViewOnly, None)
        .map_err(|e| anyhow!("failed to import account: {e:?}"))?;

    println!("  Account imported (view-only).");
    Ok(())
}
