# ZecLedger

> AI-powered Zcash accounting, reporting and payment management copilot.

ZecLedger is the first AI research and accounting tool built specifically for the Zcash ecosystem. Query the network in plain English, generate professional accounting reports, and manage ZEC payment workflows — all from your terminal.

Built in Rust. Powered by Claude AI.

## Features

### Accounting
- Full income and expense tracking across shielded and transparent transactions
- ZEC to USD conversion at time of transaction
- Net position calculation with fee breakdown
- Privacy breakdown — shielded vs transparent volume

### Reporting
- Generate reports in CSV and JSON formats
- Full ledger export with running balance
- Network-wide shield rate and adoption metrics
- Research-grade analysis powered by AI

### Payment Management
- Complete payment log with confirmation status
- Pending payment tracker with block confirmations
- Payment statistics — volume, fees, largest, smallest, average
- Multi-type support — shielded, transparent, and mixed transactions

### AI Research Copilot
- Ask any question about the Zcash network in plain English
- Powered by Claude AI with real-time blockchain context
- Research-grade answers with follow-up question suggestions

## Quick Start

### Prerequisites
- Rust (install at rustup.rs)
- Anthropic API key (console.anthropic.com)

### Install and Build
    git clone https://github.com/vancube2/zecledger.git
    cd zecledger
    cargo build --release
    export ANTHROPIC_API_KEY=sk-ant-your-key-here

## Usage

### Ask the AI Copilot
    ./target/release/zecledger ask "What does the current shield rate tell us about Zcash privacy adoption?"

### Accounting Summary
    ./target/release/zecledger accounting --blocks 100

### Payment Management
    ./target/release/zecledger payments --mode log --limit 20
    ./target/release/zecledger payments --mode pending
    ./target/release/zecledger payments --mode stats

### Generate Reports
    ./target/release/zecledger report --format csv --output report.csv
    ./target/release/zecledger report --format json --output report.json
    ./target/release/zecledger full-report --format json --output full_report.json

### Terminal Dashboard
    ./target/release/zecledger dashboard

## Roadmap
- Live lightwalletd gRPC integration
- PDF report generation
- Multi-wallet and team address tracking
- Scheduled payment automation
- Web dashboard UI
- Pay-per-report in ZEC

## Built For

ZecHub Hackathon 2026 — Accounting Track
Reporting, workflows for teams handling ZEC, payment management system.

## License

MIT
