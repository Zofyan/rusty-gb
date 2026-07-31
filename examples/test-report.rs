//! Runs every vendored test ROM and reports what each one does.
//!
//! ```sh
//! cargo run --release --example test-report              # everything
//! cargo run --release --example test-report -- mooneye   # one suite
//! cargo run --release --example test-report -- --rust    # as #[ignore] lines
//! ```
//!
//! This is the tool that maintains the `#[ignore]` attributes in `tests/`. The
//! test files name each ROM explicitly so that a regression names the ROM that
//! broke, which means the list of known failures is hand-written -- and a
//! hand-written list rots. `--rust` prints the list this run would produce, so
//! after a fix the correct move is to run it and diff, rather than to guess.
//!
//! Unlike the tests, this catches panics: an unimplemented mapper aborts the
//! ROM that hit it, not the survey.

use rusty_gb::testrom::{self, Outcome, Run, BUDGET_BLARGG_SECS, BUDGET_MOONEYE_SECS};
use std::collections::BTreeSet;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

/// A ROM that panicked the emulator outright -- an unimplemented mapper, an
/// unimplemented opcode. Reported separately from a ROM that ran and failed,
/// because the two need different fixes.
const CRASHED: &str = "crashed";

struct Report {
    name: String,
    run: Option<Run>,
    panic: Option<String>,
}

impl Report {
    fn verdict(&self) -> &str {
        match (&self.run, &self.panic) {
            (_, Some(_)) => CRASHED,
            (Some(run), _) => match run.outcome {
                Outcome::Passed => "pass",
                Outcome::Failed(_) => "fail",
                Outcome::TimedOut => "timeout",
            },
            (None, None) => unreachable!(),
        }
    }

    fn passed(&self) -> bool {
        self.verdict() == "pass"
    }
}

/// The `#[test]` function name for a ROM, derived from its path so that
/// `cargo test mem_timing` selects the obvious set.
///
/// Every path in both suites starts with a letter, so nothing here has to guard
/// against an identifier beginning with a digit.
fn ident(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.trim_end_matches(".gb").chars() {
        let ch = if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { '_' };
        if ch != '_' || !out.ends_with('_') {
            out.push(ch);
        }
    }
    out.trim_matches('_').to_string()
}

/// Why a ROM is expected to fail, in terms of what the emulator does not do
/// rather than in terms of the symptom.
///
/// Grouped by directory because the causes group that way: a whole directory of
/// sound tests fails for one reason, and repeating that reason 13 times by hand
/// is how an ignore list goes stale. Anything not covered here gets the bare
/// verdict, which is the signal to look at it and add a line.
fn cause(suite: &str, name: &str) -> Option<&'static str> {
    let gap = match suite {
        "blargg" => match name {
            n if n.starts_with("dmg_sound") => "no APU",
            n if n.starts_with("mem_timing") => "memory timing is instruction-granular",
            n if n.starts_with("oam_bug") => "OAM bug not emulated",
            "halt_bug.gb" => "halt bug not emulated",
            n if n.starts_with("interrupt_time") => "interrupt timing is instruction-granular",
            _ => return None,
        },
        "mooneye" => match name {
            n if n.starts_with("acceptance/boot_") || n.starts_with("misc/boot_") => {
                "no boot ROM: post-boot state is only approximated"
            }
            n if n.starts_with("acceptance/timer/") => "timer is instruction-granular",
            n if n.starts_with("acceptance/ppu/") || n.starts_with("misc/ppu/") => {
                "PPU is stepped per instruction, not per dot"
            }
            n if n.starts_with("acceptance/oam_dma") => "OAM DMA is not cycle-stepped",
            n if n.starts_with("emulator-only/mbc5") => "MBC5 not implemented",
            n if n.starts_with("emulator-only/") => "mapper edge cases",
            n if n.starts_with("misc/") => "CGB/AGB hardware, this is a DMG",
            n if n.starts_with("madness/") => "needs cycle-exact OAM DMA and sprites",
            // The bulk of `acceptance/` is instruction timing measured against
            // a cycle-stepped machine, which this interpreter is not.
            n if n.starts_with("acceptance/") => "instruction timing is not cycle-stepped",
            _ => return None,
        },
        _ => return None,
    };
    Some(gap)
}

/// Runs one ROM with panics contained.
fn survey(name: String, path: &Path, protocol: fn(&Path, u64) -> Run) -> Report {
    // The emulator panics on unimplemented hardware and those messages are
    // noise here -- the verdict column already says `crashed`.
    let budget = if protocol == testrom::mooneye as fn(&Path, u64) -> Run {
        BUDGET_MOONEYE_SECS
    } else {
        BUDGET_BLARGG_SECS
    };
    let previous = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let result = panic::catch_unwind(AssertUnwindSafe(|| protocol(path, budget)));
    panic::set_hook(previous);

    match result {
        Ok(run) => Report { name, run: Some(run), panic: None },
        Err(cause) => {
            let message = cause
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| cause.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_else(|| "panicked".to_string());
            Report { name, run: None, panic: Some(message) }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let as_rust = args.iter().any(|a| a == "--rust");
    let filters: Vec<&str> = args.iter().filter(|a| !a.starts_with("--")).map(|s| s.as_str()).collect();
    let wanted = |suite: &str| filters.is_empty() || filters.iter().any(|f| suite.contains(f));

    let mut reports: Vec<(&str, Vec<Report>)> = Vec::new();

    if wanted("blargg") {
        let root = PathBuf::from(testrom::BLARGG_ROOT);
        let out = testrom::blargg_roms()
            .into_iter()
            .map(|rom| survey(testrom::rom_name(&rom), &root.join(&rom), testrom::blargg))
            .collect();
        reports.push(("blargg", out));
    }

    if wanted("mooneye") {
        let root = PathBuf::from(testrom::MOONEYE_ROOT);
        let out = testrom::mooneye_roms()
            .into_iter()
            .map(|rom| survey(testrom::rom_name(&rom), &root.join(&rom), testrom::mooneye))
            .collect();
        reports.push(("mooneye", out));
    }

    if as_rust {
        let mut used = BTreeSet::new();
        for (suite, out) in &reports {
            println!("// ---- {suite}: {}/{} passing ----", out.iter().filter(|r| r.passed()).count(), out.len());
            for report in out {
                let ident = ident(&report.name);
                assert!(used.insert(ident.clone()), "two ROMs map to `{ident}`");
                if report.passed() {
                    println!("    {ident}, {:?};", report.name);
                } else {
                    let reason = match cause(suite, &report.name) {
                        Some(gap) => format!("{gap} ({})", report.verdict()),
                        None => report.verdict().to_string(),
                    };
                    println!("    {ident}, {:?}, {reason:?};", report.name);
                }
            }
        }
        return;
    }

    let mut total = 0;
    let mut passing = 0;
    for (suite, out) in &reports {
        println!("\n=== {suite} ===");
        for report in out {
            match (&report.run, &report.panic) {
                (Some(run), _) => println!("{}", testrom::summarise(&report.name, run)),
                (None, Some(message)) => {
                    println!("{:<52} {:<9} {}", report.name, CRASHED, message.lines().next().unwrap_or(""))
                }
                _ => unreachable!(),
            }
        }
        let pass = out.iter().filter(|r| r.passed()).count();
        println!("-- {suite}: {pass}/{} passing", out.len());
        total += out.len();
        passing += pass;
    }
    println!("\ntotal: {passing}/{total} passing");
}
