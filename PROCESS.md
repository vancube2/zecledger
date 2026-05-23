cat > PROCESS.md << 'EOF'
# ZecLedger — Process Documentation

## What We Built
ZecLedger is an AI-powered Zcash accounting, reporting and payment management copilot. It connects to the Zcash mainnet via a Zebra full node, analyzes real blockchain data, and lets researchers and teams query it in plain English using Claude AI.

## System Architecture
User (CLI)
↓
ZecLedger Rust Application
↓
┌─────────────────────────────────────┐
│           Core Modules              │
│  accounting · payments · reporting  │
│  copilot · output · data            │
└─────────────────────────────────────┘
↓                    ↓
Zebra Full Node      Claude AI API
(Zcash Mainnet)      (Research Copilot)
↓
Real Transaction Data
## How It Works

### 1. Data Layer
ZecLedger connects to a locally running Zebra full node via JSON-RPC. It fetches real blocks and transactions directly from Zcash mainnet, detecting transaction types (shielded, transparent, mixed) from on-chain data.

### 2. Accounting Engine
Raw transactions are processed into:
- Income vs expense tracking
- Running balance ledger
- Fee analysis
- Privacy breakdown (shielded vs transparent volume)
- Net position in ZEC and USD

### 3. Payment Management
Every transaction is tracked as a payment with:
- Confirmation status (confirmed vs pending)
- Block confirmation count
- Payment ID for team reference
- Full audit trail exportable to CSV/JSON

### 4. AI Research Copilot
Real network data is sent as context to Claude AI. Researchers ask questions in plain English and receive research-grade answers with specific numbers, pattern analysis, and follow-up research suggestions.

### 5. Reporting
Clean exports in CSV and JSON for:
- Individual transaction ledgers
- Network summary statistics
- Full combined reports (accounting + payments + network)

## Development Process

### Day 1
- Scaffolded full Rust project with 5 core modules
- Built mock data layer for development without waiting for node sync
- Integrated Claude AI copilot with Zcash network context
- CSV and JSON report generation working end to end

### Day 2
- Built accounting module with full P&L summary
- Built payment management with confirmation tracking
- Built reporting module with full combined reports
- Enabled Zebra RPC and connected to live mainnet
- Switched from mock data to real mainnet transactions

### Day 3
- Mainnet integration tested and verified
- 634 real Zcash transactions analyzed
- AI Copilot providing research-grade analysis on real data
- GitHub repo published and documented

## Tech Stack
- **Language:** Rust
- **Blockchain:** Zcash mainnet via Zebra full node (v4.4.1)
- **RPC:** Zebra JSON-RPC on port 8232
- **AI:** Claude Sonnet (Anthropic API)
- **Exports:** CSV, JSON
- **UI:** Terminal CLI + TUI dashboard

## Running ZecLedger

### Requirements
- Rust (rustup.rs)
- Zebra node running locally
- Anthropic API key

### Commands
```bash
# Accounting
zecledger accounting --blocks 100

# Payment management
zecledger payments --mode log
zecledger payments --mode pending
zecledger payments --mode stats

# AI Copilot
zecledger ask "What is the Zcash shield rate trend?"

# Reports
zecledger report --format csv
zecledger full-report --format json

# Dashboard
zecledger dashboard
```

## What Makes ZecLedger Unique
1. First AI copilot purpose-built for Zcash network research
2. Real mainnet data — not an explorer wrapper
3. Full accounting suite designed for teams handling ZEC
4. Privacy-aware — tracks shielded vs transparent volume separately
5. Pay-per-report model planned for sustainability

## Hackathon Track
ZecHub Hackathon 2026 — Accounting Track
Reporting, workflows for teams handling ZEC, payment management system.
EOF