mod cli;
mod core;
mod wallet;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("zecledger=info")
        .init();

    let cli = Cli::parse();
    let (network, endpoint) = core::config::resolve_network(cli.testnet, cli.mainnet);

    match cli.command {
        Commands::Balance => {
            wallet::show_balance(network).await?;
        }
        Commands::Sync => {
            wallet::sync(network, endpoint).await?;
        }
        Commands::WalletReport { output } => {
            wallet::generate_report(output, network).await?;
        }
        Commands::WalletAsk { question } => {
            wallet::wallet_ask(&question, network).await?;
        }
        Commands::History => {
            wallet::show_history(network).await?;
        }
        Commands::Config { show } => {
            if show { core::config::show()?; }
        }
    }
    Ok(())
}
