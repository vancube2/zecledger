# ZecLedger Local Design

The local, private side of ZecLedger. This document is the agreed scope and the
plan we build against. It is deliberately read-only and privacy-first.

## What this is

ZecLedger Local is a tool you run on your own machine to see and account for your
own shielded Zcash. It reads your activity using a viewing key, computes
everything locally, and never has the ability to move your funds.

It pairs with ZecLedger Web, the public, no-keys dashboard. The web app is for
public network data and transparent addresses. The local tool is for anything
that touches your keys.

## Core principles

- **Read-only.** The tool uses viewing keys only. It never holds a spending key
  and can never move a coin. Sending is handed off to an audited wallet.
- **Keys never leave the machine.** The viewing key is never sent to any server.
  It is stored locally in the wallet database, because that is how the underlying
  light-client crates scan for your notes. Encrypting that database at rest is
  the next planned change.
- **The server learns nothing.** Block data is fetched from a public lightwalletd
  endpoint and decrypted locally. The endpoint never sees the key, the balance,
  or which wallet is being synced (beyond IP and requested block ranges).
- **Open and reproducible.** Built on the same Rust light-client libraries the
  core ecosystem maintains, so behaviour can be checked, not just trusted.

## What it will and will not do

It will: import a viewing key, sync from lightwalletd, show shielded balances per
pool and transparent balance, list transaction history, produce accounting
reports, and answer questions about your own wallet through a local copilot.

It will not: hold spending keys, sign transactions, broadcast transactions, or
store any key on disk. Payments are completed in a separate audited wallet.

## Architecture

```
You (viewing key, entered on first sync, stored in the local wallet db)
        |
   ZecLedger Local (Rust)
        |  trial-decrypts blocks locally
        v
   public lightwalletd  (default: zec.rocks, user-overridable)
        |
   Zcash network
```

Built on the Rust light-client crates: `zcash_client_backend`,
`zcash_client_sqlite`, `zcash_keys`, `zcash_protocol`. These are the same
foundations used by production wallets such as Zashi.

A local database stores the synced block and note metadata needed to compute
balances and history, and also the viewing key itself, which `zcash_client_sqlite`
requires in order to trial-decrypt blocks on every sync. The database never leaves
the machine. It is currently unencrypted, so anyone who can read that file can see
the wallet's transaction history. It cannot be used to spend. Encrypting the
database at rest is the next planned change.

## Privacy model in plain terms

- The viewing key lets the tool see incoming and outgoing shielded notes. It
  cannot spend.
- lightwalletd serves still-encrypted compact blocks. Decryption happens on your
  machine. The server cannot read your activity.
- Metadata caveat: a public lightwalletd can see your IP and which block ranges
  you ask for. It cannot see balances, keys, or transactions. For maximum
  metadata privacy, point the tool at your own lightwalletd or route through Tor.
- The copilot is the only path where data can leave the machine, because it sends
  questions to an external API. It sends only the specific summary the user is
  asking about, and it says so clearly each time.

## Endpoint configuration

- Default: `zec.rocks` (public, community-run, speaks the compact-block protocol).
- User-overridable: paste any lightwalletd endpoint.
- Designed to swap later to a local lightwalletd, a Zaino instance, or a
  Tor-routed endpoint, in line with where the Z3 stack (Zebra, Zaino, Zallet) is
  heading.

## Key handling, exactly

1. On first sync, the tool asks for a Unified Full Viewing Key (UFVK).
2. The key is imported into the local wallet database by `zcash_client_sqlite`,
   which stores it so it can trial-decrypt blocks on this and every later sync.
3. The key is used to trial-decrypt blocks locally. It is never sent anywhere.
4. Because the key is on disk, anyone who can read the wallet database can see
   this wallet's transaction history. It cannot be used to spend. Encrypting the
   database at rest is the next planned change.

A Unified Full Viewing Key is used (not an incoming-only key) so the tool can see
both received and spent notes, which accounting requires. It still cannot spend.

## Phases

### Phase 1: Shielded balance (the centerpiece)

- Enter UFVK (memory-only, with reminder).
- Sync from lightwalletd starting at the wallet birthday height.
- Trial-decrypt locally.
- Show: shielded balance split by pool (Sapling and Orchard), plus transparent
  balance, plus a clear total.

This is the hardest single step. Wiring the light-client sync lifecycle, block
scanning, the wallet database, and viewing-key import is the bulk of the work.
Once it is done, the rest is comparatively quick because the decryption layer is
already in place. First sync from an early birthday height can take a while; the
tool shows clear sync progress.

### Phase 2: History, reports, and the copilot

- Transaction history (received and spent) from the same decrypted data.
- Accounting reports: monthly summaries, totals in and out, reconciliation.
- Local copilot answering questions about your own wallet, sending only the
  specific summary asked for, with a transparency note each time.

### Phase 3: Payment tracking and handoff

- Track and reconcile incoming payments and invoices (still read-only).
- Generate ZIP-321 payment requests and wallet deep-links so the actual send
  happens in an audited wallet. ZecLedger never holds a spending key.

## Definitions used throughout (consistent with the research)

- Fully private (z to z): value stays shielded end to end. Includes
  Sapling to Sapling, Orchard to Orchard, and cross-pool Sapling to Orchard. No
  transparent address is touched.
- Shielding (t to z): transparent in, shielded out.
- Deshielding (z to t): shielded in, transparent out.

## Open items for later

- Whether to persist an encrypted, opt-in cache to shorten re-sync time, designed
  so nothing sensitive is exposed if the disk is read.
- Tor / own-lightwalletd / Zaino endpoint support.
- Exact copilot summary format and consent prompt wording.

## Out of scope, deliberately

Spending keys, transaction signing, transaction broadcasting, and on-disk key
storage. These are the responsibility of audited wallets, and keeping them out is
what lets ZecLedger Local be trusted quickly.
