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
        Commands::Expect { amount, reference, from } => {
            wallet::expect_payment(amount, &reference, &from, network)?;
        }
        Commands::Reconcile => {
            wallet::reconcile_payments(network)?;
        }
        Commands::Expected => {
            wallet::list_expected(network)?;
        }
        Commands::Request { address, amount, memo, label, message } => {
            wallet::make_payment_request(
                &address,
                amount,
                memo.as_deref(),
                label.as_deref(),
                message.as_deref(),
            )?;
        }
        Commands::Config { show } => {
            if show { core::config::show()?; }
        }
    }
    Ok(())
}
