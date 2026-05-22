use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "zecledger", about = "AI-powered Zcash accounting, reporting & payment management copilot", version = "0.1.0")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
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
    /// Manage configuration
    Config {
        #[arg(short, long)]
        show: bool,
    },
}
