// src/wallet/costbasis.rs
//
// Cost-basis and gain/loss reporting. Reads the wallet's transaction history,
// treats received amounts as acquisitions and sent amounts as disposals, and
// matches disposals against acquisition lots using a chosen method (FIFO, LIFO,
// or average cost). For each disposal it computes proceeds minus cost-basis and
// reports the holding period in days. Prices come from a local override file by
// default (fully private); with --fetch-prices it will fetch missing daily
// prices from CoinGecko, cache them locally, and let manual overrides win.
//
// No jurisdiction assumptions are made: the report shows the holding period in
// days and leaves short/long-term classification to the user and their advisor.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use zcash_protocol::consensus::Network;

use super::history::read_history;

/// Cost-basis matching method.
#[derive(Debug, Clone, Copy)]
pub enum Method {
    Fifo,
    Lifo,
    Average,
}

impl std::str::FromStr for Method {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "fifo" => Ok(Method::Fifo),
            "lifo" => Ok(Method::Lifo),
            "average" | "avg" => Ok(Method::Average),
            other => Err(anyhow!(
                "unknown method '{other}' (use fifo, lifo, or average)"
            )),
        }
    }
}

/// A locally stored price map: date (YYYY-MM-DD) -> USD price per ZEC.
/// Manual overrides and fetched prices share this structure; overrides win.
#[derive(Debug, Default, Serialize, Deserialize)]
struct PriceStore {
    prices: HashMap<String, f64>,
}

fn prices_path(data_dir: &Path, network: Network, manual: bool) -> PathBuf {
    let base = match (network, manual) {
        (Network::TestNetwork, true) => "prices.manual.testnet.json",
        (Network::TestNetwork, false) => "prices.cache.testnet.json",
        (_, true) => "prices.manual.json",
        (_, false) => "prices.cache.json",
    };
    data_dir.join(base)
}

fn load_prices(path: &Path) -> PriceStore {
    if !path.exists() {
        return PriceStore::default();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn save_prices(path: &Path, store: &PriceStore) -> Result<()> {
    let text = serde_json::to_string_pretty(store).context("could not serialize prices")?;
    std::fs::write(path, text).with_context(|| format!("could not write {}", path.display()))?;
    Ok(())
}

fn zats_to_zec(z: i64) -> f64 {
    z as f64 / 1e8
}

/// Fetch one day's ZEC/USD price from CoinGecko. Date in YYYY-MM-DD.
/// Returns None on any failure (network, rate limit, missing data) so the
/// caller can fall back gracefully.
async fn fetch_price(date_ymd: &str) -> Option<f64> {
    // CoinGecko's /coins/{id}/history wants DD-MM-YYYY.
    let parts: Vec<&str> = date_ymd.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let dmy = format!("{}-{}-{}", parts[2], parts[1], parts[0]);
    let url = format!(
        "https://api.coingecko.com/api/v3/coins/zcash/history?date={dmy}&localization=false"
    );
    let resp = reqwest::Client::new()
        .get(&url)
        .header("accept", "application/json")
        .header("User-Agent", "zecledger/0.1 (Zcash accounting tool)")
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    json.get("market_data")?
        .get("current_price")?
        .get("usd")?
        .as_f64()
}

/// Resolve a USD price for a date: manual override first, then cache, then
/// (if fetching is enabled) CoinGecko, caching any fetched value.
async fn price_for_date(
    date_ymd: &str,
    manual: &PriceStore,
    cache: &mut PriceStore,
    fetch: bool,
    cache_path: &Path,
) -> Option<f64> {
    if let Some(p) = manual.prices.get(date_ymd) {
        return Some(*p);
    }
    if let Some(p) = cache.prices.get(date_ymd) {
        return Some(*p);
    }
    if fetch {
        if let Some(p) = fetch_price(date_ymd).await {
            cache.prices.insert(date_ymd.to_string(), p);
            let _ = save_prices(cache_path, cache);
            // be polite to the free tier
            tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
            return Some(p);
        }
    }
    None
}

/// One acquisition lot: amount of ZEC still available, its date, and unit cost.
#[derive(Debug, Clone)]
struct Lot {
    date_ymd: String,
    remaining_zec: f64,
    unit_cost_usd: Option<f64>,
}

/// A computed disposal result.
struct DisposalResult {
    date_ymd: String,
    amount_zec: f64,
    proceeds_usd: Option<f64>,
    cost_basis_usd: Option<f64>,
    holding_days: Option<i64>,
}

fn days_between(acq_ymd: &str, disp_ymd: &str) -> Option<i64> {
    let acq = chrono::NaiveDate::parse_from_str(acq_ymd, "%Y-%m-%d").ok()?;
    let disp = chrono::NaiveDate::parse_from_str(disp_ymd, "%Y-%m-%d").ok()?;
    Some((disp - acq).num_days())
}

/// Generate and print the cost-basis report.
pub async fn report(
    data_dir: &Path,
    network: Network,
    method: Method,
    fetch: bool,
    passphrase: &str,
) -> Result<()> {
    let rows = read_history(data_dir, network, passphrase)?;

    let manual = load_prices(&prices_path(data_dir, network, true));
    let cache_path = prices_path(data_dir, network, false);
    let mut cache = load_prices(&cache_path);

    // Build acquisitions and disposals in chronological order.
    let mut events: Vec<(&_, i64, String)> = Vec::new();
    for r in &rows {
        let date = r
            .time
            .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
            .map(|dt| dt.format("%Y-%m-%d").to_string());
        if let Some(d) = date {
            events.push((r, r.balance_delta, d));
        }
    }
    // Oldest first for lot building.
    events.sort_by_key(|(_, _, d)| d.clone());

    let mut lots: Vec<Lot> = Vec::new();
    let mut disposals: Vec<DisposalResult> = Vec::new();
    let mut missing_price = false;

    for (_r, delta, date) in &events {
        let price = price_for_date(date, &manual, &mut cache, fetch, &cache_path).await;
        if price.is_none() {
            missing_price = true;
        }
        if *delta >= 0 {
            // acquisition
            let amount = zats_to_zec(*delta);
            lots.push(Lot {
                date_ymd: date.clone(),
                remaining_zec: amount,
                unit_cost_usd: price,
            });
        } else {
            // disposal: consume lots by method
            let mut to_dispose = zats_to_zec(-*delta);
            let proceeds = price.map(|p| p * to_dispose);
            let mut cost_basis = Some(0.0_f64);
            let mut earliest_acq: Option<String> = None;

            // choose lot order
            let order: Vec<usize> = match method {
                Method::Fifo => (0..lots.len()).collect(),
                Method::Lifo => (0..lots.len()).rev().collect(),
                Method::Average => (0..lots.len()).collect(),
            };

            if let Method::Average = method {
                // average unit cost across all remaining lots that have a price
                let total_rem: f64 = lots.iter().map(|l| l.remaining_zec).sum();
                let total_cost: f64 = lots
                    .iter()
                    .filter_map(|l| l.unit_cost_usd.map(|c| c * l.remaining_zec))
                    .sum();
                let avg_unit = if total_rem > 0.0 {
                    total_cost / total_rem
                } else {
                    0.0
                };
                // consume proportionally
                let mut remaining = to_dispose;
                for l in lots.iter_mut() {
                    if remaining <= 0.0 {
                        break;
                    }
                    let take = remaining.min(l.remaining_zec);
                    l.remaining_zec -= take;
                    remaining -= take;
                    if earliest_acq.is_none() {
                        earliest_acq = Some(l.date_ymd.clone());
                    }
                }
                cost_basis = Some(avg_unit * to_dispose);
                to_dispose = 0.0;
            } else {
                for idx in order {
                    if to_dispose <= 0.0 {
                        break;
                    }
                    let l = &mut lots[idx];
                    if l.remaining_zec <= 0.0 {
                        continue;
                    }
                    let take = to_dispose.min(l.remaining_zec);
                    if earliest_acq.is_none() {
                        earliest_acq = Some(l.date_ymd.clone());
                    }
                    match (cost_basis, l.unit_cost_usd) {
                        (Some(cb), Some(uc)) => cost_basis = Some(cb + uc * take),
                        _ => cost_basis = None, // a consumed lot had no price
                    }
                    l.remaining_zec -= take;
                    to_dispose -= take;
                }
            }

            let holding_days = earliest_acq
                .as_ref()
                .and_then(|acq| days_between(acq, date));

            disposals.push(DisposalResult {
                date_ymd: date.clone(),
                amount_zec: zats_to_zec(-*delta),
                proceeds_usd: proceeds,
                cost_basis_usd: cost_basis,
                holding_days,
            });
        }
    }

    // Print report.
    println!();
    println!(
        "  Cost-basis report ({})",
        match method {
            Method::Fifo => "FIFO",
            Method::Lifo => "LIFO",
            Method::Average => "average cost",
        }
    );
    println!("  {:-<78}", "");
    if disposals.is_empty() {
        println!("  No disposals (sent transactions) found, so there are no realized gains yet.");
        println!("  Acquisitions tracked: {}", lots.len());
        println!("  {:-<78}", "");
        if !fetch {
            println!("  Prices come from your local override file. Use --fetch-prices to fetch missing ones.");
        }
        return Ok(());
    }

    println!(
        "  {:<12}  {:>14}  {:>14}  {:>14}  {:>10}",
        "Disposal", "Amount ZEC", "Proceeds USD", "Gain/Loss USD", "Held days"
    );
    println!("  {:-<78}", "");

    let mut total_gain = 0.0_f64;
    let mut total_gain_known = true;
    for d in &disposals {
        let gain = match (d.proceeds_usd, d.cost_basis_usd) {
            (Some(p), Some(c)) => Some(p - c),
            _ => None,
        };
        if let Some(g) = gain {
            total_gain += g;
        } else {
            total_gain_known = false;
        }
        let proceeds_s = d
            .proceeds_usd
            .map(|p| format!("{:.2}", p))
            .unwrap_or_else(|| "?".to_string());
        let gain_s = gain
            .map(|g| format!("{:.2}", g))
            .unwrap_or_else(|| "price?".to_string());
        let held_s = d
            .holding_days
            .map(|h| h.to_string())
            .unwrap_or_else(|| "-".to_string());
        println!(
            "  {:<12}  {:>14.8}  {:>14}  {:>14}  {:>10}",
            d.date_ymd, d.amount_zec, proceeds_s, gain_s, held_s
        );
    }

    println!("  {:-<78}", "");
    if total_gain_known {
        println!("  Total realized gain/loss: {:.2} USD", total_gain);
    } else {
        println!("  Total realized gain/loss: incomplete (some dates had no price)");
    }
    println!("  Holding period is shown in days; short vs long-term depends on your jurisdiction.");
    if !fetch {
        println!("  Prices come from your local override file. Use --fetch-prices to fetch missing ones.");
    }
    if missing_price {
        println!(
            "  Some dates had no price. Add them to the manual prices file or use --fetch-prices."
        );
    }
    Ok(())
}
