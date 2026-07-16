> ⚠️ **OFFICIAL REPO ONLY**: github.com/vancube2/zecledger
> ZecLedger has **NO token**. We will **NEVER** ask for ZEC, wallet access, or seed phrases. Report scams in the ZecHub Discord.

---

# ZecLedger

**Read-only Zcash shielded accounting from your viewing key.**

ZecLedger is an open-source command-line tool for doing real accounting on shielded Zcash funds. It works entirely from a viewing key, never a spending key, so it can read your shielded transaction history and produce books, reconciliations, cost-basis reports, and privacy checks, while remaining completely unable to move your money.

Most blockchains make every payment public forever. Zcash fixes that with shielded transactions, but that privacy creates a new problem: if your transactions are encrypted, how do you keep accounts? ZecLedger is built to answer that.

**Demo video:** https://youtu.be/7emZKHAH7TQ

## Ironwood (NU6.3) support is not in v0.1.0 yet

The Ironwood network upgrade activates on mainnet at **block height 3,428,143,
around 28 July 2026**. It introduces a new shielded pool and a v6 transaction
format, and the Orchard pool stops accepting new activity. Funds move out of
Orchard and into Ironwood.

**ZecLedger v0.1.0 was built before Ironwood and cannot see the Ironwood pool.**
It is built on `zcash_client_backend` 0.23, which has no Ironwood support. Once
you migrate funds out of Orchard, v0.1.0 will not count them, so any balance or
report it produces after that point may be wrong. It may also fail to sync across
the new transaction format.

This is stated here rather than discovered later. An accounting tool that quietly
reports the wrong number is worse than no tool at all.

**What to do:** v0.2.0 with Ironwood support is being worked on now and is
intended to land before 28 July 2026. Until then, treat v0.1.0 output as valid
only for pre-Ironwood history, and do not rely on it for balances after you
migrate funds. Watch the
[releases page](https://github.com/vancube2/zecledger/releases) for v0.2.0.

No funds are ever at risk either way. ZecLedger holds a viewing key, never a
spending key, and cannot move a coin.

## Why it is different

- **Read-only by design.** ZecLedger takes a Unified Full Viewing Key, never a spending key, so it structurally cannot move your funds. Your key stays on your machine and is never sent to any server. It is stored in your local wallet database, encrypted at rest with a passphrase only you know, so ZecLedger can scan the chain for your notes. See [SECURITY.md](SECURITY.md).
- **Cost-basis for shielded ZEC.** Computes realised gains and losses using FIFO, LIFO, or average cost, with the holding period in days. Because shielded transactions keep price data off-chain, ZecLedger lets you capture it (manually or via an optional price fetch).
- **Honest reconciliation.** When matching expected payments against received history, it flags partial matches for review instead of pretending they are confirmed.
- **Privacy by consent.** The optional copilot shows you exactly what aggregate data will leave your machine and waits for your explicit confirmation before sending anything.
- **Payments by handoff.** Generates standard ZIP-321 payment request URIs without ever touching a spending key.

## Commands

| Command | What it does |
| --- | --- |
| `sync` | Sync your wallet from a lightwalletd server (reads your viewing key) |
| `balance` | Show your shielded balance per pool (Sapling, Orchard, transparent) |
| `history` | Show transaction history, including decoded memos |
| `wallet-report` | Generate an accounting report (monthly summary plus full ledger, CSV/JSON) |
| `expect` / `reconcile` / `expected` | Record expected payments and reconcile them against received history |
| `request` | Generate a ZIP-321 payment request URI to send to a payer |
| `cost-basis` | Cost-basis and gain/loss report (`--method fifo\|lifo\|average`, optional `--fetch-prices`) |
| `privacy-check` | Analyse pool usage and amounts for privacy risks |
| `wallet-ask` | Ask a copilot about your wallet (shows the data and confirms before sending) |
| `config` | Manage configuration |

Global flags: `--testnet` and `--mainnet`.

## Install

### Download a binary (recommended)

No toolchain, no compiler. Grab the archive for your platform from the
[latest release](https://github.com/vancube2/zecledger/releases/latest):

| Platform | Archive |
|---|---|
| macOS (Apple Silicon) | `zecledger-<version>-aarch64-apple-darwin.tar.gz` |
| macOS (Intel) | `zecledger-<version>-x86_64-apple-darwin.tar.gz` |
| Linux (x86_64) | `zecledger-<version>-x86_64-unknown-linux-gnu.tar.gz` |
| Windows (x86_64) | `zecledger-<version>-x86_64-pc-windows-msvc.zip` |

```bash
tar xzf zecledger-<version>-<target>.tar.gz
cd zecledger-<version>-<target>
./zecledger --help
```

Optionally move it onto your PATH, for example `sudo mv zecledger /usr/local/bin/`.

### Verify what you downloaded

This tool reads your viewing key, so please check you got the real thing rather
than taking our word for it. Download `SHA256SUMS` from the same release, then:

```bash
sha256sum -c SHA256SUMS
```

Every release is also built by a public GitHub Actions workflow with
cryptographic build provenance, which proves the binary came from this repository
and this source. If you have the GitHub CLI:

```bash
gh attestation verify zecledger-<version>-<target>.tar.gz -R vancube2/zecledger
```

Only ever download ZecLedger from this repository. Anything else claiming to be
ZecLedger is fake.

### Build from source

If you would rather compile it yourself, you need a recent Rust toolchain and
`protoc` (the Protocol Buffers compiler, used to generate the lightwalletd client).

```bash
git clone https://github.com/vancube2/zecledger
cd zecledger
cargo build --release
cargo install --path .
```

This installs a `zecledger` command on your PATH.

## Quick start

```bash
# 1. Sync your wallet (you will be prompted for your viewing key and birthday height)
zecledger sync

# 2. See your shielded balance
zecledger balance

# 3. Review your transaction history with memos
zecledger history

# 4. Produce a cost-basis report
zecledger cost-basis --method fifo --fetch-prices
```

On first run ZecLedger asks for your Unified Full Viewing Key and your wallet birthday (the block height the wallet was created at). The key is used for that session only and is never stored.

The copilot (`wallet-ask`) is optional and requires an `ANTHROPIC_API_KEY` in your environment. It only ever sends aggregate totals, never addresses or memos, and only after you confirm.

## Design principles

- The core is strictly read-only. ZecLedger uses viewing keys and never spending keys, so it cannot send funds.
- Payments happen by handoff. ZecLedger produces a ZIP-321 request; your own wallet performs any actual send.
- Your viewing key never leaves your machine and is never sent to any server. It is stored in the local wallet database, encrypted at rest with SQLCipher using your passphrase, which ZecLedger never stores.
- Anything that sends data off the machine (the optional copilot) is opt-in and shown to you first.

See [DESIGN.md](DESIGN.md) for the architecture and [SECURITY.md](SECURITY.md) for the security model.

## Honest limitations

- Cost-basis output is an estimate to discuss with a professional, not a filed tax figure. Tax rules vary by country; ZecLedger reports the holding period in days and makes no jurisdiction assumptions.
- The average-cost method uses a running-average interpretation, and the holding period for a multi-lot disposal uses the earliest consumed lot.
- The privacy check only inspects pool usage and amounts. It cannot see address reuse or timing patterns, so a clean report is not a guarantee.
- Reconciliation matches on memo and amount; a clean confirmation needs the reference to appear in a memo.

## Built for

The ZecHub Hackathon 2026, Accounting track: practical accounting workflows for people and teams handling ZEC.

## License

MIT. See [LICENSE](LICENSE).
