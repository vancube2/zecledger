// src/wallet/passphrase.rs
//
// Reading the database passphrase. Input is hidden and never echoed, and the
// passphrase is never written anywhere by ZecLedger.

use anyhow::{anyhow, Context, Result};

/// Lets scripts and CI supply the passphrase without a prompt.
const ENV_VAR: &str = "ZECLEDGER_PASSPHRASE";

const MIN_LEN: usize = 8;

/// The passphrase for this process, once the user has given it.
///
/// A single run of ZecLedger is one sitting at the keyboard. Asking again for
/// every command inside that sitting is friction with no security value: the
/// person is right there, and the passphrase is already in this process's memory.
/// It is never written anywhere, and it goes when the process does.
static SESSION: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);

/// Hold onto the passphrase for the rest of this process.
pub fn remember(passphrase: &str) {
    if let Ok(mut slot) = SESSION.lock() {
        *slot = Some(passphrase.to_string());
    }
}

fn remembered() -> Option<String> {
    SESSION.lock().ok().and_then(|slot| slot.clone())
}

/// Drop the remembered passphrase.
///
/// Called when the database refuses to open, because that means what we are
/// holding is wrong. Without this, one typo would be cached and every later
/// attempt in the same run would fail against it without ever asking again.
pub fn forget() {
    if let Ok(mut slot) = SESSION.lock() {
        *slot = None;
    }
}

fn from_env() -> Option<String> {
    match std::env::var(ENV_VAR) {
        Ok(p) if !p.is_empty() => Some(p),
        _ => None,
    }
}

/// Ask for the passphrase of an existing database.
pub fn prompt_existing() -> Result<String> {
    if let Some(p) = remembered() {
        return Ok(p);
    }
    if let Some(p) = from_env() {
        return Ok(p);
    }
    let p = rpassword::prompt_password("Database passphrase: ")
        .context("could not read the passphrase")?;
    if p.is_empty() {
        return Err(anyhow!("no passphrase entered"));
    }
    remember(&p);
    Ok(p)
}

/// Ask for, and confirm, the passphrase for a brand new database.
///
/// The warning comes before the prompt on purpose: there is no recovery, and the
/// user should know that before choosing rather than after forgetting.
pub fn prompt_new() -> Result<String> {
    if let Some(p) = from_env() {
        return Ok(p);
    }
    println!();
    println!("  Choose a passphrase to encrypt your wallet database.");
    println!("  It protects your viewing key and your transaction history on disk.");
    println!();
    println!("  There is no recovery. ZecLedger does not store this passphrase and");
    println!("  cannot reset it. If you forget it, you delete the database and sync");
    println!("  again from your viewing key and birthday height. You lose no funds,");
    println!("  because ZecLedger never holds any, but you do lose the synced data.");
    println!();

    let first = rpassword::prompt_password("Choose a passphrase: ")
        .context("could not read the passphrase")?;
    if first.chars().count() < MIN_LEN {
        return Err(anyhow!(
            "passphrase must be at least {MIN_LEN} characters long"
        ));
    }
    let second = rpassword::prompt_password("Confirm passphrase: ")
        .context("could not read the passphrase")?;
    if first != second {
        return Err(anyhow!("the two passphrases did not match"));
    }
    Ok(first)
}
