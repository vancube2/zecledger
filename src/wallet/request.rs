// src/wallet/request.rs
//
// Phase 3b: ZIP-321 payment request generation. The user supplies their own
// receiving address, an amount, and optionally a memo/label/message. This
// builds a spec-compliant `zcash:` payment URI using the official zip321 crate,
// which the payer's wallet can read to pre-fill the payment. ZecLedger never
// holds keys or sends anything; it only formats the request (read-only handoff).

use anyhow::{anyhow, Context, Result};
use std::str::FromStr;

use zcash_address::ZcashAddress;
use zcash_protocol::memo::{Memo, MemoBytes};
use zcash_protocol::value::Zatoshis;
use zip321::{Payment, TransactionRequest};

/// Convert a ZEC decimal amount to Zatoshis (1 ZEC = 100,000,000 zatoshis).
fn zec_to_zatoshis(zec: f64) -> Result<Zatoshis> {
    if zec < 0.0 {
        return Err(anyhow!("amount cannot be negative"));
    }
    // Round to the nearest zatoshi to avoid float drift.
    let zats = (zec * 1e8).round() as u64;
    Zatoshis::from_u64(zats).map_err(|_| anyhow!("amount is out of range"))
}

/// Build and print a ZIP-321 payment request URI.
pub fn make_request(
    address: &str,
    amount_zec: f64,
    memo: Option<&str>,
    label: Option<&str>,
    message: Option<&str>,
) -> Result<()> {
    // Parse and validate the recipient address.
    let recipient = ZcashAddress::try_from_encoded(address)
        .map_err(|_| anyhow!("that does not look like a valid Zcash address"))?;

    let amount = zec_to_zatoshis(amount_zec)?;

    // Encode the memo text into MemoBytes if provided.
    let memo_bytes: Option<MemoBytes> = match memo {
        Some(text) if !text.is_empty() => {
            let m = Memo::from_str(text).context("memo text could not be encoded")?;
            Some(MemoBytes::from(&m))
        }
        _ => None,
    };

    let payment = Payment::new(
        recipient,
        Some(amount),
        memo_bytes,
        label.map(|s| s.to_string()),
        message.map(|s| s.to_string()),
        vec![],
    )
    .map_err(|e| anyhow!("could not build payment request: {e:?}"))?;

    let request = TransactionRequest::new(vec![payment])
        .map_err(|e| anyhow!("could not build transaction request: {e:?}"))?;

    let uri = request.to_uri();

    println!();
    println!("  Payment request (ZIP-321)");
    println!("  {:-<70}", "");
    println!("  Send this to whoever is paying you. Their wallet will read it.");
    println!();
    println!("  {uri}");
    println!("  {:-<70}", "");
    println!("  This is a request only. ZecLedger never sends funds or holds keys.");
    Ok(())
}
