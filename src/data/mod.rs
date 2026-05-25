use crate::core::{Transaction, TxType};
use anyhow::Result;
use chrono::{Duration, Utc, DateTime};
use uuid::Uuid;

const ZEBRA_RPC: &str = "http://127.0.0.1:8232";
const BLOCKCHAIR_API: &str = "https://api.blockchair.com/zcash";

pub async fn fetch_transactions(blocks: u32) -> Result<Vec<Transaction>> {
    // Try Blockchair API first for latest data
    match fetch_from_blockchair(blocks).await {
        Ok(txs) if !txs.is_empty() => {
            println!("Connected to Zcash mainnet (latest blocks) — {} transactions fetched", txs.len());
            return Ok(txs);
        }
        _ => {}
    }

    // Fallback to local Zebra
    match fetch_from_zebra(blocks).await {
        Ok(txs) if !txs.is_empty() => {
            println!("Connected to local Zebra node — {} transactions fetched", txs.len());
            return Ok(txs);
        }
        _ => {}
    }

    // Final fallback to mock data
    println!("Using mock data for development");
    Ok(generate_mock_transactions(blocks))
}

async fn fetch_from_blockchair(blocks: u32) -> Result<Vec<Transaction>> {
    let client = reqwest::Client::new();
    let limit = (blocks * 3).min(500);

    // Get latest transactions
    let url = format!(
        "{}/transactions?limit={}&s=block_id(desc)",
        BLOCKCHAIR_API, limit
    );

    let resp = client
        .get(&url)
        .header("User-Agent", "ZecLedger/0.1")
        .send().await?;

    let json: serde_json::Value = resp.json().await?;
    let txs_data = json["data"].as_array()
        .ok_or_else(|| anyhow::anyhow!("No data"))?;

    let zec_price = get_zec_price(&client).await.unwrap_or(28.50);
    let mut txs = Vec::new();

    for tx in txs_data {
        let txid = tx["hash"].as_str()
            .unwrap_or("unknown").to_string();
        let block_height = tx["block_id"].as_u64().unwrap_or(0);
        let amount_zec = tx["output_total"].as_f64()
            .unwrap_or(0.0) / 100_000_000.0;
        let fee_zec = tx["fee"].as_f64()
            .unwrap_or(10000.0) / 100_000_000.0;

        let time_str = tx["time"].as_str().unwrap_or("");
        let timestamp = DateTime::parse_from_str(
            &format!("{} +0000", time_str),
            "%Y-%m-%d %H:%M:%S %z"
        ).map(|dt| dt.with_timezone(&Utc))
         .unwrap_or_else(|_| Utc::now());

        // Detect shielded from is_coinbase and input/output counts
        let shielded_in = tx["shielded_input_count"].as_u64().unwrap_or(0);
        let shielded_out = tx["shielded_output_count"].as_u64().unwrap_or(0);
        let transparent_in = tx["input_count"].as_u64().unwrap_or(0);

        let tx_type = match (shielded_in + shielded_out > 0, transparent_in > 0) {
            (true, false)  => TxType::Shielded,
            (false, true)  => TxType::Transparent,
            (true, true)   => TxType::Mixed,
            (false, false) => TxType::Transparent,
        };

        if amount_zec > 0.0 {
            txs.push(Transaction {
                txid,
                block_height,
                timestamp,
                tx_type,
                amount_zec,
                amount_usd: Some(amount_zec * zec_price),
                fee_zec,
                memo: None,
            });
        }
    }

    Ok(txs)
}

async fn fetch_from_zebra(blocks: u32) -> Result<Vec<Transaction>> {
    let client = reqwest::Client::new();

    let height_resp: serde_json::Value = client
        .post(ZEBRA_RPC)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "getblockcount",
            "params": [],
            "id": 1
        }))
        .send().await?.json().await?;

    let tip = height_resp["result"].as_u64()
        .ok_or_else(|| anyhow::anyhow!("No block height"))?;

    let start = tip.saturating_sub(blocks as u64);
    let mut txs = Vec::new();
    let zec_price = 28.50_f64;

    for height in (start..=tip).rev().take(blocks as usize) {
        let block_resp: serde_json::Value = client
            .post(ZEBRA_RPC)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "getblock",
                "params": [height.to_string(), 2],
                "id": 1
            }))
            .send().await?.json().await?;

        let block = &block_resp["result"];
        let time = block["time"].as_i64().unwrap_or(0);
        let timestamp = DateTime::from_timestamp(time, 0)
            .unwrap_or_else(Utc::now);

        if let Some(block_txs) = block["tx"].as_array() {
            for tx in block_txs {
                let txid = tx["txid"].as_str()
                    .unwrap_or("unknown").to_string();
                let has_shielded_in = tx["vShieldedSpend"].as_array()
                    .map(|a| !a.is_empty()).unwrap_or(false);
                let has_shielded_out = tx["vShieldedOutput"].as_array()
                    .map(|a| !a.is_empty()).unwrap_or(false);
                let has_transparent = tx["vin"].as_array()
                    .map(|a| !a.is_empty()).unwrap_or(false);

                let tx_type = match (has_shielded_in || has_shielded_out, has_transparent) {
                    (true, false)  => TxType::Shielded,
                    (false, true)  => TxType::Transparent,
                    (true, true)   => TxType::Mixed,
                    (false, false) => TxType::Transparent,
                };

                let amount_zec: f64 = tx["vout"].as_array()
                    .map(|outs| outs.iter()
                        .filter_map(|o| o["value"].as_f64())
                        .sum())
                    .unwrap_or(0.0);

                if amount_zec > 0.0 {
                    txs.push(Transaction {
                        txid,
                        block_height: height,
                        timestamp,
                        tx_type,
                        amount_zec,
                        amount_usd: Some(amount_zec * zec_price),
                        fee_zec: 0.0001,
                        memo: None,
                    });
                }
            }
        }
        if txs.len() >= (blocks * 3) as usize { break; }
    }

    Ok(txs)
}

async fn get_zec_price(client: &reqwest::Client) -> Result<f64> {
    let resp = client
        .get("https://api.coingecko.com/api/v3/simple/price?ids=zcash&vs_currencies=usd")
        .send().await?;
    let json: serde_json::Value = resp.json().await?;
    Ok(json["zcash"]["usd"].as_f64().unwrap_or(28.50))
}

pub fn generate_mock_transactions(blocks: u32) -> Vec<Transaction> {
    let total = (blocks * 3) as usize;
    let mut txs = Vec::with_capacity(total);
    let now = Utc::now();
    let zec_price = 28.50_f64;
    let types = [TxType::Shielded, TxType::Shielded, TxType::Transparent, TxType::Mixed];

    for i in 0..total {
        let tx_type = types[i % types.len()].clone();
        let amount = match &tx_type {
            TxType::Shielded    => 0.5 + (i as f64 % 10.0) * 0.8,
            TxType::Transparent => 1.0 + (i as f64 % 5.0)  * 2.5,
            TxType::Mixed       => 0.25 + (i as f64 % 8.0) * 0.6,
        };
        txs.push(Transaction {
            txid: Uuid::new_v4().to_string().replace("-", ""),
            block_height: 3_354_000 + (i as u64 / 3),
            timestamp: now - Duration::minutes(i as i64 * 2),
            amount_usd: Some(amount * zec_price),
            fee_zec: 0.0001,
            memo: if matches!(tx_type, TxType::Shielded) {
                Some("ZecLedger".to_string())
            } else { None },
            tx_type,
            amount_zec: amount,
        });
    }
    txs
}

pub async fn fetch_from_blockchair_pub(blocks: u32) -> Result<Vec<Transaction>> {
    match fetch_from_blockchair(blocks).await {
        Ok(txs) if !txs.is_empty() => Ok(txs),
        _ => Ok(generate_mock_transactions(blocks)),
    }
}
