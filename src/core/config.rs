use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub lightwalletd_url: String,
    pub claude_api_key: Option<String>,
    pub data_dir: PathBuf,
    pub default_blocks: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            lightwalletd_url: "https://mainnet.lightwalletd.com:9067".to_string(),
            claude_api_key: std::env::var("ANTHROPIC_API_KEY").ok(),
            data_dir: dirs::cache_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("zecledger"),
            default_blocks: 100,
        }
    }
}

pub fn load() -> Result<Config> { Ok(Config::default()) }

pub fn show() -> Result<()> {
    let config = load()?;
    println!("lightwalletd_url = {}", config.lightwalletd_url);
    println!("default_blocks   = {}", config.default_blocks);
    println!("api_key_set      = {}", config.claude_api_key.is_some());
    Ok(())
}
