use anyhow::Result;
use serde_json::json;
use crate::data::fetch_from_blockchair_pub;
use crate::core::NetworkStats;

const SYSTEM_PROMPT: &str = "You are ZecLedger Copilot, an expert AI research assistant for the Zcash blockchain. Answer researcher questions clearly with specific numbers. Always end with one follow-up research question.";

pub async fn ask(question: &str) -> Result<()> {
    println!("\n ZecLedger Copilot");
    println!("{}", "─".repeat(50));
    println!("Q: {}\n", question);
    println!("Fetching latest mainnet data...");

    let txs = fetch_from_blockchair_pub(50).await?;
    let stats = NetworkStats::from_transactions(&txs, 3_354_054);

    let context = format!(
        "Latest Zcash Mainnet Data (May 2026 block ~3354054):\n\
         - Total transactions: {}\n\
         - Shielded: {} ({:.1}%)\n\
         - Transparent: {}\n\
         - Total volume: {:.4} ZEC\n\
         - Shield rate: {:.1}%",
        stats.total_transactions, stats.shielded_count,
        stats.shield_rate_pct, stats.transparent_count,
        stats.total_volume_zec, stats.shield_rate_pct,
    );

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("Set: export ANTHROPIC_API_KEY=sk-ant-..."))?;

    println!("Thinking...\n");

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&json!({
            "model": "claude-sonnet-4-20250514",
            "max_tokens": 1024,
            "system": SYSTEM_PROMPT,
            "messages": [{
                "role": "user",
                "content": format!("{}\n\nQuestion: {}", context, question)
            }]
        }))
        .send().await?;

    let status = response.status();
    let body: serde_json::Value = response.json().await?;

    if !status.is_success() {
        println!("API Error: {}", body);
        return Ok(());
    }

    let answer = body["content"][0]["text"].as_str().unwrap_or("No response");
    println!("Answer:");
    println!("{}", "─".repeat(50));
    println!("{}", answer);
    println!("{}", "─".repeat(50));
    println!("\nTip: zecledger report --format csv to export");
    Ok(())
}