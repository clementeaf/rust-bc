//! Tesseract interactive CLI — explore the 4D probability field.
//!
//! Run: cargo run --bin cli
//!
//! Commands:
//!   genesis <name> <amount>       — allocate initial tokens
//!   transfer <from> <to> <amount> — zero-sum transfer
//!   balance <name>                — check balance
//!   seed <t> <c> <o> <v> <id>    — seed raw event at coordinate
//!   query <t> <c> <o> <v>        — inspect cell state
//!   destroy <t> <c> <o> <v>      — destroy a cell (attack simulation)
//!   evolve [steps]               — evolve field (default: to equilibrium)
//!   status                       — field overview
//!   receipts                     — list transfer receipts
//!   verify <index>               — verify conservation proof for receipt
//!   help                         — show commands
//!   quit                         — exit

use std::io::{self, BufRead, Write};

use tesseract::wallet::{TesseractLedger, TransferRequest};
use tesseract::{evolve_to_equilibrium, Coord};

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const DIM: &str = "\x1b[2m";

fn main() {
    let field_size: usize = std::env::var("FIELD_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(16);

    let mut ledger = TesseractLedger::new(field_size);
    let mut tx_counter: u64 = 0;

    println!();
    println!("{BOLD}{CYAN}╔══════════════════════════════════════════���═══════╗{RESET}");
    println!("{BOLD}{CYAN}║   TESSERACT CLI — 4D Probability Field REPL     ║{RESET}");
    println!("{BOLD}{CYAN}╚══════════════════════════════════════════════════╝{RESET}");
    println!(
        "{DIM}  Field size: {field_size}⁴ ({} logical cells){RESET}",
        field_size.pow(4)
    );
    println!("{DIM}  Type 'help' for commands, 'quit' to exit.{RESET}");
    println!();

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("{BOLD}{CYAN}tesseract>{RESET} ");
        let _ = stdout.flush();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Err(_) => break,
            _ => {}
        }

        let parts: Vec<&str> = line.trim().split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "quit" | "exit" | "q" => {
                println!("{DIM}Goodbye.{RESET}");
                break;
            }

            "help" | "h" | "?" => print_help(),

            "genesis" | "gen" => {
                if parts.len() < 3 {
                    err("Usage: genesis <name> <amount>");
                    continue;
                }
                let name = parts[1];
                let amount: u64 = match parts[2].parse() {
                    Ok(a) => a,
                    Err(_) => {
                        err("Amount must be a positive integer");
                        continue;
                    }
                };
                ledger.genesis_allocate(name, amount, 0);
                // Lock supply after first genesis batch
                if ledger.balance(name) == 0 {
                    // genesis_allocate doesn't call conserved.genesis for single allocs
                    // Use the batch genesis instead
                }
                ok(&format!("Genesis: {} receives {} tokens", name, amount));
                info(&format!("Balance: {} = {}", name, ledger.balance(name)));
            }

            "init" => {
                // Batch genesis: init alice=1000 bob=500
                if parts.len() < 2 {
                    err("Usage: init <name>=<amount> [name=amount ...]");
                    continue;
                }
                let mut allocs = Vec::new();
                for pair in &parts[1..] {
                    let kv: Vec<&str> = pair.split('=').collect();
                    if kv.len() != 2 {
                        err(&format!("Invalid format: '{}'. Use name=amount", pair));
                        continue;
                    }
                    let amount: u64 = match kv[1].parse() {
                        Ok(a) => a,
                        Err(_) => {
                            err(&format!("Invalid amount: '{}'", kv[1]));
                            continue;
                        }
                    };
                    allocs.push((kv[0], amount, 0u64));
                }
                if allocs.is_empty() {
                    continue;
                }
                // Need to build refs for the slice
                let alloc_refs: Vec<(&str, u64, u64)> =
                    allocs.iter().map(|(n, a, t)| (*n, *a, *t)).collect();
                ledger.genesis(&alloc_refs);
                for (name, amount, _) in &allocs {
                    ok(&format!("{}: {} tokens", name, amount));
                }
                info(&format!(
                    "Supply locked. Conservation: {}",
                    if ledger.is_conserved() {
                        "OK"
                    } else {
                        "VIOLATED"
                    }
                ));
            }

            "transfer" | "tx" | "send" => {
                if parts.len() < 4 {
                    err("Usage: transfer <from> <to> <amount>");
                    continue;
                }
                let from = parts[1];
                let to = parts[2];
                let amount: u64 = match parts[3].parse() {
                    Ok(a) => a,
                    Err(_) => {
                        err("Amount must be a positive integer");
                        continue;
                    }
                };
                tx_counter += 1;
                let req = TransferRequest {
                    id: format!("tx-{:04}", tx_counter),
                    from: from.to_string(),
                    to: to.to_string(),
                    amount,
                    timestamp: tx_counter,
                    channel: "default".to_string(),
                };
                match ledger.transfer(req) {
                    Ok(receipt) => {
                        ok(&format!(
                            "{} → {} : {} tokens (tx-{:04})",
                            from, to, amount, tx_counter
                        ));
                        let verified = receipt.verify_conservation();
                        info(&format!(
                            "Pedersen proof: {}",
                            if verified { "VALID" } else { "INVALID" }
                        ));
                        info(&format!(
                            "Balances: {} = {}, {} = {}",
                            from,
                            ledger.balance(from),
                            to,
                            ledger.balance(to)
                        ));
                    }
                    Err(e) => err(&format!("Transfer failed: {}", e)),
                }
            }

            "balance" | "bal" => {
                if parts.len() < 2 {
                    err("Usage: balance <name>");
                    continue;
                }
                let b = ledger.balance(parts[1]);
                info(&format!("{}: {} tokens", parts[1], b));
            }

            "seed" => {
                if parts.len() < 6 {
                    err("Usage: seed <t> <c> <o> <v> <event_id>");
                    continue;
                }
                let coords: Result<Vec<usize>, _> = parts[1..5].iter().map(|s| s.parse()).collect();
                match coords {
                    Ok(c) => {
                        let coord = Coord {
                            t: c[0],
                            c: c[1],
                            o: c[2],
                            v: c[3],
                        };
                        let id = parts[5..].join(" ");
                        ledger.field.seed_named(coord, &id);
                        ledger.field.evolve();
                        let cell = ledger.field.get(coord);
                        ok(&format!(
                            "Seeded '{}' at {} — p={:.4}, crystallized={}",
                            id, coord, cell.probability, cell.crystallized
                        ));
                    }
                    Err(_) => err("Coordinates must be integers"),
                }
            }

            "query" | "cell" | "get" => {
                if parts.len() < 5 {
                    err("Usage: query <t> <c> <o> <v>");
                    continue;
                }
                let coords: Result<Vec<usize>, _> = parts[1..5].iter().map(|s| s.parse()).collect();
                match coords {
                    Ok(c) => {
                        let coord = Coord {
                            t: c[0],
                            c: c[1],
                            o: c[2],
                            v: c[3],
                        };
                        let cell = ledger.field.get(coord);
                        let support = ledger.field.orthogonal_support(coord);
                        println!("  {BOLD}Cell {}{RESET}", coord);
                        println!("    Probability:  {:.4}", cell.probability);
                        println!(
                            "    Crystallized: {}",
                            if cell.crystallized {
                                format!("{GREEN}true{RESET}")
                            } else {
                                format!("{DIM}false{RESET}")
                            }
                        );
                        println!("    σ-support:    {}/4", support);
                        println!("    Record:       {}", cell.record());
                    }
                    Err(_) => err("Coordinates must be integers"),
                }
            }

            "destroy" | "attack" => {
                if parts.len() < 5 {
                    err("Usage: destroy <t> <c> <o> <v>");
                    continue;
                }
                let coords: Result<Vec<usize>, _> = parts[1..5].iter().map(|s| s.parse()).collect();
                match coords {
                    Ok(c) => {
                        let coord = Coord {
                            t: c[0],
                            c: c[1],
                            o: c[2],
                            v: c[3],
                        };
                        ledger.field.destroy(coord);
                        ok(&format!("Destroyed cell {}", coord));
                    }
                    Err(_) => err("Coordinates must be integers"),
                }
            }

            "evolve" | "settle" => {
                let steps: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                if steps > 0 {
                    let mut crystallized = 0;
                    for _ in 0..steps {
                        crystallized += ledger.field.evolve();
                    }
                    ok(&format!(
                        "Evolved {} steps, {} new crystallizations",
                        steps, crystallized
                    ));
                } else {
                    evolve_to_equilibrium(&mut ledger.field, 10);
                    ok("Evolved to equilibrium");
                }
                info(&format!(
                    "Active: {}, Crystallized: {}",
                    ledger.field.active_cells(),
                    ledger.field.crystallized_count()
                ));
            }

            "status" | "stat" | "s" => {
                println!();
                println!("  {BOLD}Field Status{RESET}");
                println!("    Size:          {}⁴", ledger.field.size);
                println!("    Active cells:  {}", ledger.field.active_cells());
                println!("    Crystallized:  {}", ledger.field.crystallized_count());
                let total = ledger.field.total_cells();
                let active = ledger.field.active_cells();
                let sparsity = if total > 0 {
                    100.0 - (active as f64 / total as f64 * 100.0)
                } else {
                    100.0
                };
                println!("    Sparsity:      {:.2}%", sparsity);
                println!("    Transfers:     {}", ledger.transfer_count());
                println!(
                    "    Conservation:  {}",
                    if ledger.is_conserved() {
                        format!("{GREEN}OK{RESET}")
                    } else {
                        format!("{RED}VIOLATED{RESET}")
                    }
                );
                println!();
            }

            "receipts" | "txs" => {
                if ledger.receipts.is_empty() {
                    info("No transfers yet.");
                    continue;
                }
                println!();
                for (i, r) in ledger.receipts.iter().enumerate() {
                    let hash_short = hex::encode(&r.hash[..4]);
                    let confirmed = ledger.is_confirmed(r);
                    let status = if confirmed {
                        format!("{GREEN}confirmed{RESET}")
                    } else {
                        format!("{YELLOW}pending{RESET}")
                    };
                    println!(
                        "  [{:>3}] {} debit={} credit={} [{}]",
                        i, hash_short, r.debit_coord, r.credit_coord, status
                    );
                }
                println!();
            }

            "verify" => {
                if parts.len() < 2 {
                    err("Usage: verify <receipt_index>");
                    continue;
                }
                let idx: usize = match parts[1].parse() {
                    Ok(i) => i,
                    Err(_) => {
                        err("Index must be a number");
                        continue;
                    }
                };
                match ledger.receipts.get(idx) {
                    Some(receipt) => {
                        let valid = receipt.verify_conservation();
                        if valid {
                            ok(&format!(
                                "Receipt [{}]: Pedersen conservation proof VALID",
                                idx
                            ));
                        } else {
                            err(&format!(
                                "Receipt [{}]: Pedersen conservation proof INVALID",
                                idx
                            ));
                        }
                    }
                    None => err(&format!("No receipt at index {}", idx)),
                }
            }

            other => {
                err(&format!(
                    "Unknown command: '{}'. Type 'help' for usage.",
                    other
                ));
            }
        }
    }
}

fn print_help() {
    println!();
    println!("  {BOLD}Ledger{RESET}");
    println!("    init <name>=<amt> ...     Batch genesis allocation (locks supply)");
    println!("    genesis <name> <amount>   Single genesis allocation");
    println!("    transfer <from> <to> <n>  Zero-sum transfer with Pedersen proof");
    println!("    balance <name>            Check participant balance");
    println!("    receipts                  List all transfer receipts");
    println!("    verify <index>            Verify Pedersen proof for receipt");
    println!();
    println!("  {BOLD}Field{RESET}");
    println!("    seed <t> <c> <o> <v> <id> Seed raw event at coordinate");
    println!("    query <t> <c> <o> <v>     Inspect cell state");
    println!("    destroy <t> <c> <o> <v>   Destroy cell (attack simulation)");
    println!("    evolve [steps]            Evolve field (default: to equilibrium)");
    println!("    status                    Field overview");
    println!();
    println!("  {BOLD}Other{RESET}");
    println!("    help                      This message");
    println!("    quit                      Exit");
    println!();
}

fn ok(msg: &str) {
    println!("  {GREEN}✓ {msg}{RESET}");
}

fn err(msg: &str) {
    println!("  {RED}✗ {msg}{RESET}");
}

fn info(msg: &str) {
    println!("  {DIM}{msg}{RESET}");
}
