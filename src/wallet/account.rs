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
use zcash_protocol::consensus::Network;

use super::db::wallet_db_path;

/// Connect to lightwalletd, build the birthday, import the UFVK as view-only.
pub async fn import_view_only(
    data_dir: &Path,
    endpoint: &str,
    ufvk: &UnifiedFullViewingKey,
    birthday_height: u64,
    network: Network,
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
    let db_path = wallet_db_path(data_dir, network);
    let mut db = WalletDb::for_path(&db_path, network, SystemClock, OsRng)
        .context("failed to open wallet database")?;

    use zcash_client_backend::data_api::WalletRead;
    let existing = db.get_account_ids().unwrap_or_default();
    if existing.is_empty() {
        db.import_account_ufvk("main", ufvk, &birthday, AccountPurpose::ViewOnly, None)
            .map_err(|e| anyhow!("failed to import account: {e:?}"))?;
        println!("  Account imported (view-only).");
    } else {
        println!("  Account already present, syncing existing wallet.");
    }
    Ok(())
}

/// Step 3c/3d: scan blocks from the birthday forward, decrypting locally.
pub async fn sync_blocks(data_dir: &Path, endpoint: &str, network: Network) -> Result<()> {
    use zcash_client_sqlite::FsBlockDb;
    use zcash_client_sqlite::chain::init::init_blockmeta_db;

    println!("  Connecting to {endpoint} for sync ...");
    let mut client = CompactTxStreamerClient::connect(endpoint.to_string())
        .await
        .with_context(|| format!("could not connect to {endpoint}"))?;

    let blocks_dir = data_dir.join("blocks");
    std::fs::create_dir_all(&blocks_dir).context("could not create blocks dir")?;
    let mut fs_cache = FsBlockDb::for_path(&blocks_dir)
        .map_err(|e| anyhow!("failed to open block cache: {e:?}"))?;
    init_blockmeta_db(&mut fs_cache)
        .map_err(|e| anyhow!("failed to init block cache: {e:?}"))?;
    let inner_blocks = blocks_dir.join("blocks");
    let block_cache = super::cache::ZecLedgerCache::new(fs_cache, inner_blocks);

    let db_path = wallet_db_path(data_dir, network);
    let mut db = WalletDb::for_path(&db_path, network, SystemClock, OsRng)
        .context("failed to open wallet database")?;

    println!("  Scanning blocks (this may take a moment) ...");
    zcash_client_backend::sync::run(&mut client, &network, &block_cache, &mut db, 1000)
        .await
        .map_err(|e| anyhow!("sync failed: {e:?}"))?;

    println!("  Sync complete.");
    Ok(())
}
