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

    // Someone who runs ZecLedger with no arguments is almost always someone who
    // just downloaded it and wants to know what it is. Clap's usage error is the
    // wrong answer to that, and on a double-clicked Windows console it is not even
    // readable before the window closes. Answer the actual question instead.
    if std::env::args().count() == 1 {
        let result = cli::welcome::run().await;
        // Hold a double-clicked window open whatever happened, including on an
        // error. A window that vanishes on failure is the original bug.
        if let Err(e) = result {
            eprintln!();
            eprintln!("  Error: {e}");
            cli::welcome::pause_if_double_clicked();
            std::process::exit(1);
        }
        cli::welcome::pause_if_double_clicked();
        return Ok(());
    }

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
        Commands::Expect {
            amount,
            reference,
            from,
        } => {
            wallet::expect_payment(amount, &reference, &from, network)?;
        }
        Commands::Reconcile => {
            wallet::reconcile_payments(network)?;
        }
        Commands::Expected => {
            wallet::list_expected(network)?;
        }
        Commands::Request {
            address,
            amount,
            memo,
            label,
            message,
        } => {
            wallet::make_payment_request(
                &address,
                amount,
                memo.as_deref(),
                label.as_deref(),
                message.as_deref(),
            )?;
        }
        Commands::CostBasis {
            method,
            fetch_prices,
        } => {
            wallet::cost_basis_report(&method, fetch_prices, network).await?;
        }
        Commands::PrivacyCheck => {
            wallet::privacy_report(network)?;
        }
        Commands::Config { show } => {
            if show {
                core::config::show()?;
            }
        }
    }
    Ok(())
}
