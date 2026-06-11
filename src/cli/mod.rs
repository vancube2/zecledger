use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "zecledger", about = "AI-powered Zcash accounting, reporting & payment management copilot", version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Use Zcash testnet instead of mainnet
    #[arg(long, global = true)]
    pub testnet: bool,

    /// Force mainnet even if config sets testnet
    #[arg(long, global = true)]
    pub mainnet: bool,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Ask the AI copilot a question about the Zcash network
    Ask { question: String },
    /// Fetch recent transactions
    Fetch {
        #[arg(short, long, default_value = "100")]
        blocks: u32,
    },
    /// Generate a research report (csv, json)
    Report {
        #[arg(short, long, default_value = "csv")]
        format: String,
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Show accounting summary — income, expenses, net position
    Accounting {
        #[arg(short, long, default_value = "100")]
        blocks: u32,
        #[arg(short, long)]
        export: Option<String>,
    },
    /// Payment management — log, pending, stats
    Payments {
        #[arg(short, long, default_value = "log")]
        mode: String,
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },
    /// Generate full report with accounting + payments
    FullReport {
        #[arg(short, long, default_value = "csv")]
        format: String,
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Launch interactive terminal dashboard
    Dashboard,
    /// Show your shielded balance per pool (Sapling, Orchard, transparent)
    Balance,
    /// Sync your wallet from a lightwalletd server (reads your viewing key)
    Sync,
    /// Generate a wallet accounting report (monthly summary + full ledger)
    WalletReport {
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Ask the copilot about YOUR wallet (shows data and confirms before sending)
    WalletAsk { question: String },
    /// Show transaction history from the synced wallet
    History,
    /// Manage configuration
    Config {
        #[arg(short, long)]
        show: bool,
    },
}
