# ZecLedger, Process Documentation

How this tool was built, including the pivot that defines it.

## The short version

ZecLedger started as one thing and became another. It began as a research tool that
read the public Zcash chain and reasoned about it. Partway through, it became clear
that this was an explorer wrapper solving a problem nobody urgently had, while the
real unsolved problem sat next to it untouched: a person or team holding shielded ZEC
genuinely cannot do their own books, because the chain data is encrypted to everyone,
including them.

That is the problem privacy creates. ZecLedger was rebuilt around it.

## Version 1, May 22 to early June 2026

The first commit was a Zcash network research copilot. It connected to a locally
running Zebra full node over JSON-RPC, pulled real blocks and transactions from
mainnet, classified them as shielded, transparent, or mixed, and ran an accounting
engine over that public data. It had payment tracking, CSV and JSON audit trails, a
terminal dashboard, and a copilot that answered plain-English questions with real
network data as context. It shipped with verified raw research data across several
time windows.

That version worked. It also had a conceptual ceiling.

## The realisation

Analysing public network data is, at bottom, an explorer with a language model on
top. It is useful, but it does not solve anything that only Zcash has, and it does
not touch the actual gap: your own encrypted books.

## The pivot, June 8 2026

DESIGN.md was written on June 8. Commit 9a90096, "Add design doc for local shielded
accounting tool", is the hinge of the whole project. From that point the centre of
gravity moved:

From reading the public chain and reasoning about the network.
To reading your own shielded wallet with a viewing key, locally, and doing real
accounting on it.

The fake-data and public-analysis scaffolding was later stripped out entirely
(commit 36b04b7, "Remove fake-data scaffolding commands, keep only real wallet
pipeline"), and the README was rewritten for the tool that actually exists
(commit be3e847).

## The split the pivot produced

The pivot also produced the two-part architecture that defines ZecLedger today,
stated directly in DESIGN.md:

- **ZecLedger Local (the CLI)** handles anything that touches your keys. Viewing key
  in, shielded accounting out, everything computed on your own machine.
- **ZecLedger Web** is the public, no-keys dashboard. Public network data and
  transparent addresses only. It never asks for or handles any key.

The version 1 network-analysis work did not die. It became the web app. That is why
zecledger-web does shield-rate sampling, network and fee statistics, and transparent
address lookup. It is the public-data half, correctly separated from the private half.

## How it was built

43 commits, May 22 to June 24 2026.

**Design before code.** DESIGN.md was written first and treated as the agreed scope:
core principles, what the tool will and will not do, the architecture, the privacy
model in plain terms, exact key handling, and an explicit out-of-scope list. The code
followed the plan.

**Hardest thing first, in phases.** DESIGN.md named Phase 1, shielded balance, as the
centerpiece and the hardest single step, and predicted the rest would move quickly
once the decryption layer worked. That is what happened, and the commits are labelled
by phase and step:

- Phase 1: light-client dependencies, wallet scaffolding, viewing-key input with
  validation and birthday height, wallet database, lightwalletd connection and
  view-only account import, block sync via a custom BlockCache wrapper, and finally
  shielded balance per pool.
- Phase 2: transaction history from the v_transactions view, accounting reports with
  monthly summary and CSV/JSON export, and a wallet copilot with explicit send consent.
- Phase 3: expected-payment reconciliation on memo and amount, and ZIP-321 payment
  request generation.
- Then the value-add reports: cost basis and gain/loss with FIFO, LIFO and average,
  and a privacy-hygiene report over pool usage and amount patterns.

**One step, one commit.** Every commit is a single working step with a plain-English
message. Build one thing, test it against real data, commit, move on.

**Security from the start, not bolted on.** Security and scam-defence commits appear
early and repeatedly, before the pivot: a security policy and scam warning, a
hardened .gitignore, tightly pinned dependencies, and dependency vulnerability fixes.

**Honesty passes.** Several commits exist purely to remove things that should not be
there or to stop misleading the user: removing accidentally committed output files,
removing the fake-data scaffolding, clearing dead code, and rewriting the README for
the current tool.

## What the tool is now

Viewing-key shielded accounting over lightwalletd, in Rust, built on the same
light-client crates the ecosystem maintains (zcash_client_backend, zcash_client_sqlite,
zcash_keys). The key is stored in the local wallet database, because those crates need
it to trial-decrypt blocks on every sync, and it never leaves the machine. That database
is encrypted at rest with SQLCipher using a passphrase ZecLedger never stores. The tool
is structurally unable to spend: it takes a Unified Full Viewing Key,
never a spending key, and payments are handed off as ZIP-321 requests to the user's own
audited wallet.

Current commands: `sync`, `balance`, `history`, `wallet-report`, `expect` / `reconcile`
/ `expected`, `request`, `cost-basis`, `privacy-check`, `wallet-ask`, `config`, with
`--testnet` and `--mainnet` flags.

## Submission

Built for the ZecHub Hackathon 2026, Accounting track: reporting and workflows for
teams handling ZEC, and payment management.

Companion work in the same ecosystem push: the ZecHub wiki cost-basis PR #1774.
