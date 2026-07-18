// src/wallet/db.rs
//
// The wallet database: where it lives, how it is opened, and how it is encrypted.
//
// The database holds your viewing key and your decrypted transaction history, so it
// is encrypted at rest with SQLCipher using a passphrase only you know. ZecLedger
// never stores that passphrase anywhere.

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use rand::rngs::OsRng;
use rusqlite::Connection;
use zcash_client_sqlite::util::SystemClock;
use zcash_client_sqlite::WalletDb;
use zcash_protocol::consensus::Network;

/// The concrete wallet database type used throughout ZecLedger.
pub type ZecWalletDb = WalletDb<Connection, Network, SystemClock, OsRng>;

/// Where the wallet database lives, under the configured data dir.
pub fn wallet_db_path(data_dir: &Path, network: Network) -> PathBuf {
    match network {
        Network::TestNetwork => data_dir.join("wallet.testnet.sqlite"),
        _ => data_dir.join("wallet.sqlite"),
    }
}

/// Stop SQLCipher printing its internals straight to stderr.
///
/// A wrong passphrase makes SQLCipher log three lines about hmac checks failing
/// and pages not decrypting. That is exactly right for a developer and useless
/// and frightening for everyone else: it reads like the wallet is corrupt, when
/// in fact a character was mistyped. ZecLedger already says so in plain words, so
/// route these into tracing, where anyone who wants them can ask with RUST_LOG.
pub fn quiet_sqlite_logging() {
    // Safe here because it runs once, before any connection is opened.
    unsafe {
        let _ = rusqlite::trace::config_log(Some(|code, msg| {
            tracing::debug!("sqlite {code}: {msg}");
        }));
    }
}

/// Quote a value as a SQL string literal.
///
/// SQLCipher parses the KEY clause of ATTACH before bound parameters are applied,
/// so that one value has to be inlined. Doubling any single quote keeps it safe.
fn sql_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

/// Open a connection to the wallet database with the SQLCipher key applied,
/// verified, and the array module loaded ready for `zcash_client_sqlite`.
pub fn open_conn(db_path: &Path, passphrase: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("could not open {}", db_path.display()))?;
    conn.pragma_update(None, "key", passphrase)
        .context("could not apply the database key")?;
    verify_readable(db_path, &conn)?;
    rusqlite::vtab::array::load_module(&conn).context("could not load the sqlite array module")?;
    Ok(conn)
}

/// SQLCipher accepts any key without complaint and only fails when data is first
/// read. Force that read so a wrong passphrase becomes a clear message instead of
/// a confusing failure later on.
fn verify_readable(db_path: &Path, conn: &Connection) -> Result<()> {
    if conn
        .query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
        .is_ok()
    {
        return Ok(());
    }
    if is_plaintext(db_path) {
        return Err(anyhow!(
            "this wallet database is not encrypted yet. Run 'zecledger sync' to encrypt it."
        ));
    }
    // Whatever passphrase we just used did not work, so stop holding onto it.
    // Otherwise an interactive session would keep reusing a wrong one silently.
    super::passphrase::forget();
    Err(anyhow!(
        "could not read the wallet database. The passphrase is wrong, or this file is not a ZecLedger database."
    ))
}

/// Open the wallet database as a `WalletDb`, encrypted.
pub fn open_wallet_db(data_dir: &Path, network: Network, passphrase: &str) -> Result<ZecWalletDb> {
    let db_path = wallet_db_path(data_dir, network);
    let conn = open_conn(&db_path, passphrase)?;
    Ok(WalletDb::from_connection(conn, network, SystemClock, OsRng))
}

/// True if the database exists and can be read with no key at all, which means it
/// predates encryption and the viewing key inside it is sitting in the clear.
pub fn is_plaintext(db_path: &Path) -> bool {
    if !db_path.exists() {
        return false;
    }
    match Connection::open(db_path) {
        Ok(conn) => conn
            .query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
            .is_ok(),
        Err(_) => false,
    }
}

/// Encrypt an existing plaintext database in place. The old file is kept with a
/// `.plaintext.bak` extension so nothing is destroyed without the user knowing.
/// Returns the path of that backup.
pub fn encrypt_in_place(db_path: &Path, passphrase: &str) -> Result<PathBuf> {
    let tmp = db_path.with_extension("encrypting");
    if tmp.exists() {
        fs::remove_file(&tmp).ok();
    }

    {
        let conn = Connection::open(db_path).context("could not open the existing database")?;
        conn.execute_batch(&format!(
            "ATTACH DATABASE {} AS encrypted KEY {};",
            sql_quote(&tmp.to_string_lossy()),
            sql_quote(passphrase)
        ))
        .context("could not attach a new encrypted database")?;
        conn.query_row("SELECT sqlcipher_export('encrypted')", [], |_| Ok(()))
            .context("could not copy the data into the encrypted database")?;
        conn.execute_batch("DETACH DATABASE encrypted;")
            .context("could not detach the encrypted database")?;
    }

    let backup = db_path.with_extension("sqlite.plaintext.bak");
    fs::rename(db_path, &backup).context("could not move the old plaintext database aside")?;
    fs::rename(&tmp, db_path).context("could not put the encrypted database in place")?;
    Ok(backup)
}

/// Open (creating if needed) and initialize the wallet database, encrypted.
pub fn open_and_init(data_dir: &Path, network: Network, passphrase: &str) -> Result<()> {
    fs::create_dir_all(data_dir)
        .with_context(|| format!("could not create data dir {}", data_dir.display()))?;

    let db_path = wallet_db_path(data_dir, network);
    println!("  Wallet database: {}", db_path.display());

    let mut db = open_wallet_db(data_dir, network, passphrase)?;

    zcash_client_sqlite::wallet::init::init_wallet_db(&mut db, None)
        .context("failed to initialize wallet database schema")?;

    println!("  Wallet database ready, encrypted at rest.");
    Ok(())
}

/// Whether the wallet database already holds an imported account. If it does, the
/// viewing key is already there and the user does not need to paste it again.
pub fn has_account(data_dir: &Path, network: Network, passphrase: &str) -> Result<bool> {
    let db_path = wallet_db_path(data_dir, network);
    if !db_path.exists() {
        return Ok(false);
    }
    use zcash_client_backend::data_api::WalletRead;
    let db = open_wallet_db(data_dir, network, passphrase)?;
    Ok(!db.get_account_ids().unwrap_or_default().is_empty())
}
