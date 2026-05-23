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

## Setup & Installation

### Step 1 — Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

### Step 2 — Install and run Zebra node
cargo install --locked zebrad
zebrad generate -o ~/.config/zebrad.toml
sed -i '/^\[rpc\]/a listen_addr = "127.0.0.1:8232"' ~/.config/zebrad.toml
sed -i 's/enable_cookie_auth = true/enable_cookie_auth = false/' ~/.config/zebrad.toml
nohup zebrad start > /tmp/zebra.log 2>&1 &

### Step 3 — Clone and build ZecLedger
git clone https://github.com/vancube2/zecledger.git
cd zecledger
cargo build --release

### Step 4 — Set API key and run
export ANTHROPIC_API_KEY=your_key_here
./target/release/zecledger --help
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
