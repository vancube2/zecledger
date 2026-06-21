use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "zecledger", about = "Read-only Zcash shielded accounting from your viewing key", version = "0.1.0")]
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
    /// Sync your wallet from a lightwalletd server (reads your viewing key)
    Sync,
    /// Show your shielded balance per pool (Sapling, Orchard, transparent)
    Balance,
    /// Show transaction history from the synced wallet
    History,
    /// Generate a wallet accounting report (monthly summary + full ledger)
    WalletReport {
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Ask the copilot about YOUR wallet (shows data and confirms before sending)
    WalletAsk { question: String },
    /// Record a payment you are expecting to receive
    Expect {
        #[arg(short, long)]
        amount: f64,
        #[arg(short, long)]
        reference: String,
        #[arg(long)]
        from: String,
    },
    /// Check expected payments against your received history
    Reconcile,
    /// List the payments you are expecting
    Expected,
    /// Manage configuration
    Config {
        #[arg(short, long)]
        show: bool,
    },
}
