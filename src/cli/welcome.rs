// src/cli/welcome.rs
//
// What someone sees when they run ZecLedger with no arguments.
//
// This exists because of a real failure. On Windows, double-clicking the program
// opens a console, clap prints a usage error, and the console closes instantly.
// The program looks broken when it is working perfectly. Nobody should have to
// read documentation to discover that a program is a command line program, so
// this says so, tells them what to type, and waits before the window disappears.

use std::path::PathBuf;

/// True when the program was double-clicked from a file manager rather than run
/// from a terminal, which means the window will vanish the moment we return.
///
/// On Windows a console launched for a double-click has exactly one process
/// attached: this one. A console the user already had open has at least two,
/// because the shell is attached too.
#[cfg(windows)]
fn launched_by_double_click() -> bool {
    use windows_sys::Win32::System::Console::GetConsoleProcessList;
    let mut pids = [0u32; 2];
    let count = unsafe { GetConsoleProcessList(pids.as_mut_ptr(), 2) };
    count == 1
}

/// Elsewhere, double-clicking either does not open a terminal at all or leaves it
/// open, so there is nothing to work around.
#[cfg(not(windows))]
fn launched_by_double_click() -> bool {
    false
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

/// Which network has a wallet on this machine, if any.
///
/// This returns the network rather than a bare yes or no, because knowing that a
/// wallet exists is useless if you then go and open the wrong one. Mainnet wins
/// when both are present, matching what the plain commands default to.
fn existing_wallet_network() -> Option<zcash_protocol::consensus::Network> {
    use zcash_protocol::consensus::Network;
    let cfg = crate::core::config::load().ok()?;
    if crate::wallet::db::wallet_db_path(&cfg.data_dir, Network::MainNetwork).exists() {
        Some(Network::MainNetwork)
    } else if crate::wallet::db::wallet_db_path(&cfg.data_dir, Network::TestNetwork).exists() {
        Some(Network::TestNetwork)
    } else {
        None
    }
}

/// Which network is the viewing key for?
///
/// This has to be asked before the key, because a key carries its network in it
/// and will be rejected against the wrong one. Assuming mainnet and letting a
/// testnet user hit a confusing error is not a real default.
fn choose_network() -> bool {
    use std::io::{IsTerminal, Write};
    if !std::io::stdin().is_terminal() {
        return false;
    }
    println!();
    println!("  Which network is your viewing key for?");
    println!("    1. Mainnet, the real Zcash network. This is almost always the answer.");
    println!("    2. Testnet, for testing with coins that are not worth anything.");
    println!();
    print!("  Choose 1 or 2 (default 1): ");
    let _ = std::io::stdout().flush();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    // true means testnet
    answer.trim() == "2"
}

/// Ask a yes or no question, defaulting to yes on a bare Enter.
fn confirm(question: &str) -> bool {
    use std::io::{IsTerminal, Write};
    // Never prompt when there is nobody there to answer, for example when piped
    // into another program or run from a script.
    if !std::io::stdin().is_terminal() {
        return false;
    }
    print!("  {question} (Y/n) ");
    let _ = std::io::stdout().flush();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return false;
    }
    let answer = answer.trim().to_lowercase();
    answer.is_empty() || answer == "y" || answer == "yes"
}

/// Read one line of free text, trimmed. Returns an empty string when there is no
/// terminal (piped or scripted) or on read error, so callers can treat empty as
/// "no answer given" and skip rather than block.
fn prompt(question: &str) -> String {
    use std::io::{IsTerminal, Write};
    if !std::io::stdin().is_terminal() {
        return String::new();
    }
    print!("  {question} ");
    let _ = std::io::stdout().flush();
    let mut answer = String::new();
    if std::io::stdin().read_line(&mut answer).is_err() {
        return String::new();
    }
    answer.trim().to_string()
}

/// What happens when ZecLedger is run with no arguments.
///
/// Someone in this position has usually just installed it and wants to start, so
/// offer to start rather than printing a list of commands and leaving them to work
/// out which one comes first.
pub async fn run() -> anyhow::Result<()> {
    banner();

    if let Some(found) = existing_wallet_network() {
        // Open the wallet they actually have. Defaulting to mainnet here meant a
        // testnet user got an empty mainnet database and a wall of "no such table".
        let testnet = matches!(found, zcash_protocol::consensus::Network::TestNetwork);
        let (network, endpoint) = crate::core::config::resolve_network(testnet, !testnet);
        if testnet {
            println!("  Using your testnet wallet.");
        }
        menu(network, endpoint).await?;
        println!();
        commands();
        if launched_by_double_click() {
            terminal_help();
        }
        return Ok(());
    }

    // No wallet yet. This is someone's first run.
    println!("  No wallet has been set up on this computer yet.");
    println!();
    println!("  Setting one up means pasting a viewing key, choosing a passphrase to");
    println!("  encrypt the local database, and letting it read the chain. Nothing");
    println!("  leaves this machine, and a viewing key cannot move funds.");
    println!();

    // A double-clicked console is a real console. You can paste into it and type
    // in it, and the window is held open when we are done. There is no reason to
    // send someone away to a different terminal just to answer three questions.
    if confirm("Set one up now?") {
        // Ask the network first, then hand off with a matching endpoint. Getting
        // this wrong points the sync at the wrong servers as well as the wrong key.
        let testnet = choose_network();
        let (network, endpoint) = crate::core::config::resolve_network(testnet, !testnet);
        println!();
        crate::wallet::sync(network, endpoint.clone()).await?;

        println!();
        println!("  Your wallet is set up and synced. You can use it right here.");

        // Straight into the menu in the same window. They are already sitting in
        // front of a working ZecLedger; sending them elsewhere to read their own
        // balance would be silly.
        menu(network, endpoint).await?;

        println!();
        commands();
        if launched_by_double_click() {
            terminal_help();
        }
        return Ok(());
    }

    println!();
    println!("  No problem. When you are ready, run:");
    println!("    zecledger sync");
    println!();
    commands();
    Ok(())
}

fn banner() {
    let version = env!("CARGO_PKG_VERSION");
    println!();
    println!("  ZecLedger {version}");
    println!("  Read-only Zcash shielded accounting from your viewing key.");
    println!();
    println!("  This is a command line program, so it has no window of its own.");
    println!("  You use it by typing commands in a terminal.");
    println!();
}

/// Where to go next, for the commands that a single window cannot cover.
fn terminal_help() {
    {
        println!("  To run any of these, open a terminal:");
        println!();
        println!("    1. Open PowerShell.");
        println!("       Press the Windows key, type powershell, and press Enter.");
        println!();
        println!("    2. Go to this folder by typing this, with the quotes:");
        match exe_dir() {
            Some(dir) => println!("         cd \"{}\"", dir.display()),
            None => println!("         cd \"the folder you extracted ZecLedger into\""),
        }
        println!();
        println!("    3. Then run it, for example:");
        println!("         .\\zecledger.exe balance");
        println!();
        println!("  Better still, put it on your PATH so you can just type");
        println!("  zecledger from anywhere.");
        println!();
    }
}

fn commands() {
    println!("  What it can do");
    println!();
    println!("    Getting started");
    println!("      sync              enter your viewing key and synchronise the wallet");
    println!("      config --show     show where your data is kept");
    println!();
    println!("    Your wallet");
    println!("      balance           shielded balance for each pool");
    println!("      history           transactions the viewing key can see");
    println!();
    println!("    Accounting");
    println!("      wallet-report     monthly summary and full ledger, CSV and JSON");
    println!("      cost-basis        gains and losses, using fifo, lifo or average");
    println!();
    println!("    Payments");
    println!("      request           make a payment request to send to a payer");
    println!("      expect            record a payment you are expecting");
    println!("      reconcile         match what arrived against what you expected");
    println!();
    println!("    Privacy");
    println!("      privacy-check     what your own wallet data reveals");
    println!();
    println!("    Copilot");
    println!("      wallet-ask        ask an AI about your own wallet data");
    println!();
    println!("  For the full list and every option:");
    println!("    zecledger --help");
    println!();
    println!("  ZecLedger never asks for a seed phrase or spending key, and cannot");
    println!("  move funds. Only ever download it from:");
    println!("    https://github.com/vancube2/zecledger");
    println!();
}

/// Hold the window open when there is no terminal to return to, so the message is
/// actually readable instead of flashing past.
pub fn pause_if_double_clicked() {
    if !launched_by_double_click() {
        return;
    }
    println!("  Press Enter to close this window.");
    let mut discard = String::new();
    let _ = std::io::stdin().read_line(&mut discard);
}

/// A menu for the window you are already in.
///
/// The point of this is simple: if ZecLedger is open and your wallet is synced,
/// you should be able to do your accounting right here. Telling someone to go and
/// open a different terminal to read their own balance is not an answer.
async fn menu(network: zcash_protocol::consensus::Network, endpoint: String) -> anyhow::Result<()> {
    use std::io::{IsTerminal, Write};
    if !std::io::stdin().is_terminal() {
        return Ok(());
    }

    loop {
        println!();
        println!("  What would you like to do?");
        println!();
        println!("    1. Balance");
        println!("    2. History");
        println!("    3. Accounting report (choose CSV, JSON, or Markdown)");
        println!("    4. Cost basis, gains and losses");
        println!("    5. Privacy check");
        println!("    6. Expected payments, and reconcile them");
        println!("    7. Sync again");
        println!("    8. Ask the copilot");
        println!("    0. Quit");
        println!();
        print!("  Choose: ");
        let _ = std::io::stdout().flush();

        let mut choice = String::new();
        if std::io::stdin().read_line(&mut choice).is_err() {
            return Ok(());
        }
        let choice = choice.trim();

        // A failing command should not throw the user out of the menu. Report it
        // and let them try something else.
        let outcome: anyhow::Result<()> = match choice {
            "1" => crate::wallet::show_balance(network).await,
            "2" => crate::wallet::show_history(network).await,
            "3" => {
                // Let them choose which file(s) they want. The report is saved
                // with an automatic name and the guide tells them where; asking
                // for a filename would expose plumbing no one cares about.
                use crate::wallet::report::ReportFormat;
                let fmt = match prompt("Format - 1) CSV  2) JSON  3) both  4) Markdown [both]:")
                    .to_lowercase()
                    .as_str()
                {
                    "1" | "csv" => ReportFormat::Csv,
                    "2" | "json" => ReportFormat::Json,
                    "4" | "markdown" | "md" => ReportFormat::Markdown,
                    _ => ReportFormat::Both,
                };
                crate::wallet::generate_report_choice(network, fmt).await
            }
            "4" => {
                // Expose what the cost-basis command already accepts instead of
                // silently assuming fifo.
                let method = match prompt("Method - 1) fifo  2) lifo  3) average [fifo]:")
                    .to_lowercase()
                    .as_str()
                {
                    "2" | "lifo" => "lifo",
                    "3" | "average" | "avg" => "average",
                    _ => "fifo",
                };
                let fetch = {
                    let a = prompt("Fetch live market prices? (y/N):").to_lowercase();
                    a == "y" || a == "yes"
                };
                crate::wallet::cost_basis_report(method, fetch, network).await
            }
            "5" => crate::wallet::privacy_report(network),
            "6" => crate::wallet::reconcile_payments(network),
            "7" => crate::wallet::sync(network, endpoint.clone()).await,
            "8" => {
                // The copilot sends a summary of local data to the Anthropic API,
                // which needs a key. Check for it before prompting, so a new user
                // is told how to set one up instead of typing a question and only
                // then hitting an error.
                if std::env::var("ANTHROPIC_API_KEY").is_err() {
                    println!();
                    println!("  The copilot needs an Anthropic API key, which is not set yet.");
                    println!("  Get one at https://console.anthropic.com, then set it:");
                    if cfg!(windows) {
                        println!("    setx ANTHROPIC_API_KEY \"sk-ant-...\"");
                        println!("  then close and reopen this window so it takes effect.");
                    } else {
                        println!("    export ANTHROPIC_API_KEY=sk-ant-...");
                    }
                    Ok(())
                } else {
                    let q = prompt("Ask about your wallet:");
                    if q.is_empty() {
                        Ok(())
                    } else {
                        crate::wallet::wallet_ask(&q, network).await
                    }
                }
            }
            "0" | "q" | "quit" | "exit" => return Ok(()),
            "" => continue,
            other => {
                println!("  '{other}' is not one of the choices.");
                continue;
            }
        };

        if let Err(e) = outcome {
            println!();
            println!("  That did not work: {e}");
        }
    }
}
