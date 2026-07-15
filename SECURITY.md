# Security Policy

## Official Repository
The ONLY official ZecLedger repository is:
https://github.com/vancube2/zecledger

Any other repository, website, or app claiming to be ZecLedger is fake.
Report fakes to ZecHub Discord immediately.

## ZecLedger Will NEVER:
- Ask you to send ZEC to any address
- Ask for your seed phrase or private keys
- Ask you to connect your wallet
- Charge for access to the CLI tool
- DM you on Discord, Twitter, or Telegram
- Offer a token, airdrop, or investment opportunity

## If Someone Claims Otherwise:
It is a scam. Block and report them.

## Reporting Security Vulnerabilities
If you find a genuine security bug:
- Do NOT open a public GitHub issue
- Open a private GitHub security advisory at:
  https://github.com/vancube2/zecledger/security/advisories/new
- We will respond within 48 hours.

## What ZecLedger Does NOT Do:
- Hold or transmit user funds
- Ask for or store a spending key or seed phrase
- Connect to user wallets
- Execute transactions on behalf of users

Your funds are always safe in your own wallet. ZecLedger takes a viewing key,
never a spending key, so it is structurally incapable of moving a coin.

## Where your viewing key is stored

Your Unified Full Viewing Key is stored on your own machine, in the local wallet
database in your data directory. This is required: `zcash_client_sqlite` needs the
key in order to trial-decrypt blocks and find your notes, on the first sync and on
every sync after it. Every wallet built on these crates works this way.

That database is **encrypted at rest** with SQLCipher. On your first sync you choose
a passphrase, and the database cannot be read without it.

What that means in practice:

- Your key never leaves your machine and is never sent to any server.
- The key cannot spend. It is a viewing key, so no funds are at risk from it.
- Without your passphrase the database is unreadable, including by us.
- ZecLedger never stores your passphrase and cannot reset or recover it. If you
  forget it, delete the database and sync again from your viewing key and birthday
  height. You lose the synced data, never any funds.
- `ZECLEDGER_PASSPHRASE` can supply the passphrase for scripts. If you use it, that
  value is in your environment, so treat it accordingly.

If you synced with a version of ZecLedger from before encryption, your database is
plaintext. Run `zecledger sync` and it will detect that, encrypt it, and leave the
old unencrypted file beside it with a `.plaintext.bak` extension so you can check
the result and then delete it. That backup still contains your key in the clear.

If you want to remove everything, delete your ZecLedger data directory. The key and
all synced data go with it.

## Disclaimer
ZecLedger is an open-source MIT-licensed research tool
provided as-is without warranty. Use at your own risk.
