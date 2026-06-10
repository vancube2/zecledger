mod cli;
mod core;
mod data;
mod output;
mod copilot;
mod accounting;
mod payments;
mod reporting;
mod wallet;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("zecledger=info")
        .init();
    let cli = Cli::parse();
    match cli.command {
        Commands::Ask { question } => {
            copilot::ask(&question).await?;
        }
        Commands::Fetch { blocks } => {
            let txs = data::fetch_transactions(blocks).await?;
            println!("Fetched {} transactions", txs.len());
        }
        Commands::Report { format, output } => {
            info!("Generating {} report...", format);
            let txs = data::fetch_transactions(100).await?;
            output::generate(&txs, &format, output.as_deref()).await?;
        }
        Commands::Accounting { blocks, export } => {
            let txs = data::fetch_transactions(blocks).await?;
            let summary = accounting::generate_summary(&txs);
            accounting::print_summary(&summary);
            if let Some(path) = export {
                let json = serde_json::to_string_pretty(&summary)?;
                std::fs::write(&path, json)?;
                println!("Accounting data exported to: {}", path);
            }
        }
        Commands::Payments { mode, limit } => {
            let txs = data::fetch_transactions(100).await?;
            let payment_log = payments::build_payment_log(&txs);
            match mode.as_str() {
                "log"     => payments::print_payment_log(&payment_log, limit),
                "pending" => payments::print_pending(&payment_log),
                "stats"   => {
                    let stats = payments::get_stats(&payment_log);
                    payments::print_stats(&stats);
                }
                _ => println!("Use --mode log, pending, or stats"),
            }
        }
        Commands::FullReport { format, output } => {
            info!("Generating full report...");
            let txs = data::fetch_transactions(200).await?;
            let default_name = format!(
                "zecledger_full_{}.{}",
                chrono::Utc::now().format("%Y%m%d_%H%M%S"),
                format
            );
            let path = output.unwrap_or(default_name);
            reporting::generate_full_report(&txs, &path, &format)?;
        }
        Commands::Dashboard => {
            output::tui::run().await?;
        }
        Commands::Balance => {
            wallet::show_balance().await?;
        }
        Commands::Sync => {
            wallet::sync().await?;
        }
        Commands::WalletReport { output } => {
            wallet::generate_report(output).await?;
        }
        Commands::History => {
            wallet::show_history().await?;
        }
        Commands::Config { show } => {
            if show { core::config::show()?; }
        }
    }
    Ok(())
}
