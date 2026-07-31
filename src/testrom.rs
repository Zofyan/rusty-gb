//! Running a test ROM to completion and reading back its verdict.
//!
//! Test ROMs report their result to the machine, not to a screen, and the two
//! suites this repository carries do it in two different ways:
//!
//! * **blargg** reports through one of two channels depending on its vintage.
//!   The `cpu_instrs` generation writes its report to the serial port and ends
//!   it with `Passed` or `Failed`; [`Emulator::step`] already drains SB into
//!   whatever sink it is given, so reading that verdict is reading a string.
//!   Everything later -- `dmg_sound`, `oam_bug`, `mem_timing-2`, `halt_bug`,
//!   `interrupt_time` -- says nothing over serial at all and instead leaves a
//!   status byte and its report text in cartridge RAM. Watching only the serial
//!   port makes all twenty of those look like hangs when they have in fact
//!   finished and are sitting in their exit loop, so [`blargg`] watches both.
//! * **mooneye** stops at a `LD B,B` software breakpoint with the result in the
//!   register file: the first six Fibonacci numbers for a pass, `0x42` in every
//!   register for a failure. Nothing reaches the serial port that a `Write` sink
//!   could catch, so this one has to single-step and watch the opcode.
//!
//! Both end by spinning forever rather than halting, so every runner here takes
//! a cycle budget and gives up on it.
//!
//! Host-only: it wants `std` for the filesystem and for wall-clock timing, and
//! the Pico has no test ROMs to run anyway.

use crate::emulator::Emulator;
use crate::input;
use crate::output::dummy::Dummy;
use crate::output::{Output, PX_COLOR, PX_PALETTE, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::rom::Rom;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// M-cycles a DMG executes in one second: the 4.194304 MHz dot clock over four
/// dots to the M-cycle.
pub const CYCLES_PER_SECOND: u64 = 1_048_576;

/// Budget for a blargg ROM, in emulated seconds. The combined `cpu_instrs.gb`
/// is the slowest thing in either suite at around 54, so this is headroom over
/// that rather than a round number.
pub const BUDGET_BLARGG_SECS: u64 = 90;

/// Budget for a mooneye ROM, in emulated seconds. These are short: the slowest
/// that finishes at all is a mapper test at just over three seconds. Kept tight
/// because the budget is only ever spent in full by a ROM that has hung, and
/// there are enough of those to notice.
pub const BUDGET_MOONEYE_SECS: u64 = 10;

/// What a ROM reported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    Passed,
    /// The ROM said it failed. The string is whatever detail the protocol
    /// carried -- blargg names the failing sub-test, mooneye has only the
    /// register file to offer.
    Failed(String),
    /// The budget ran out before the ROM said anything either way. Usually an
    /// unimplemented feature the ROM is waiting on rather than a wrong answer.
    TimedOut,
}

impl Outcome {
    pub fn passed(&self) -> bool {
        matches!(self, Outcome::Passed)
    }
}

/// The result of one run, including what it cost.
#[derive(Clone, Debug)]
pub struct Run {
    pub outcome: Outcome,
    /// Everything the ROM wrote to the serial port, verbatim.
    pub serial: String,
    /// Emulated M-cycles consumed before the verdict landed.
    pub cycles: u64,
    /// Wall-clock time the run took.
    pub elapsed: Duration,
}

impl Run {
    /// Panics unless the ROM passed, with the ROM's own output as the message.
    ///
    /// This is what the `#[test]` functions call: a failing test should print
    /// the report the ROM produced, because that names the sub-test that broke
    /// far better than an assertion on a boolean would.
    #[track_caller]
    pub fn assert_passed(&self) {
        assert!(
            self.outcome.passed(),
            "{:?} after {} cycles ({:.2}s emulated)\n--- serial ---\n{}\n--------------",
            self.outcome,
            self.cycles,
            self.cycles as f64 / CYCLES_PER_SECOND as f64,
            if self.serial.is_empty() { "(nothing)" } else { &self.serial },
        );
    }
}

/// Steps `emu` until `verdict` returns something or the budget runs out.
///
/// `verdict` runs every `poll_every` instructions rather than every one, so
/// that a check too expensive to afford per instruction -- reading four bus
/// addresses, scanning a string -- is still affordable. Both protocols below
/// end with the ROM spinning forever on its result, so a late poll sees the
/// same answer an immediate one would; `poll_every` of 1 is for the one check
/// that genuinely has to be exact.
fn drive<F>(rom: &Path, budget_secs: u64, poll_every: u32, mut verdict: F) -> Run
where
    F: FnMut(&Emulator<input::Dummy, Dummy>, &str) -> Option<Outcome>,
{
    let mut emu = Emulator::new(
        Rom::file(rom.to_str().expect("rom path is not utf-8")),
        input::Dummy::new(),
        Dummy::new(),
    );

    let budget = budget_secs * CYCLES_PER_SECOND;
    let mut serial = String::new();
    let started = Instant::now();
    let mut until_poll = 0u32;

    let outcome = loop {
        if until_poll == 0 {
            if let Some(outcome) = verdict(&emu, &serial) {
                break outcome;
            }
            until_poll = poll_every;
        }
        until_poll -= 1;

        if emu.cycles() >= budget {
            // One last look, so a ROM that finished inside the final poll
            // interval is not reported as a hang.
            break verdict(&emu, &serial).unwrap_or(Outcome::TimedOut);
        }
        emu.step(&mut serial);
    };

    Run { outcome, serial, cycles: emu.cycles(), elapsed: started.elapsed() }
}

/// Where blargg's later ROMs leave their result: a status byte, a signature so
/// a reader can tell the protocol is live, then NUL-terminated report text.
mod blargg_ram {
    pub const STATUS: u16 = 0xA000;
    pub const SIGNATURE: u16 = 0xA001;
    /// Present once the ROM has claimed the protocol. Cartridge RAM reads back
    /// as zeroes before that, so this doubles as "the run has started".
    pub const MAGIC: [u8; 3] = [0xDE, 0xB0, 0x61];
    pub const TEXT: u16 = 0xA004;
    /// `STATUS` while the test is still running. Any other value is final.
    pub const RUNNING: u8 = 0x80;
    /// `STATUS` on success. Anything else is the number of the check that
    /// failed, which is the `#N` the report text quotes.
    pub const PASSED: u8 = 0x00;
    /// Enough for the longest report in the suite (halt_bug's table) with room
    /// to spare, and short enough to stay inside the 8 KiB RAM window.
    pub const TEXT_LIMIT: u16 = 0x800;
}

/// Runs a blargg ROM until either of its report channels carries a verdict.
///
/// The serial report is scanned from a high-water mark rather than in full:
/// these ROMs emit a few hundred bytes over tens of millions of instructions,
/// and re-scanning the whole string on every poll made the search the dominant
/// cost of the run.
pub fn blargg(rom: &Path, budget_secs: u64) -> Run {
    // Longest needle, less one, so a verdict split across two polls is still
    // seen whole.
    const OVERLAP: usize = "Failed".len() - 1;
    let mut scanned = 0usize;

    drive(rom, budget_secs, 512, move |emu, serial| {
        if let Some(outcome) = blargg_cart_ram(emu) {
            return Some(outcome);
        }

        if serial.len() <= scanned {
            return None;
        }
        let from = scanned.saturating_sub(OVERLAP);
        let tail = &serial[from..];
        scanned = serial.len();

        // "Failed" first: a run that fails one sub-test of many still prints
        // per-test lines, and the final line is the one that decides.
        if tail.contains("Failed") {
            Some(Outcome::Failed(serial.trim().to_string()))
        } else if tail.contains("Passed") {
            Some(Outcome::Passed)
        } else {
            None
        }
    })
}

/// Reads the cartridge-RAM report, if the ROM has finished writing one.
fn blargg_cart_ram<O: Output>(emu: &Emulator<input::Dummy, O>) -> Option<Outcome> {
    use blargg_ram::*;

    let signature = [
        emu.peek_at(SIGNATURE),
        emu.peek_at(SIGNATURE + 1),
        emu.peek_at(SIGNATURE + 2),
    ];
    if signature != MAGIC {
        return None;
    }
    let status = emu.peek_at(STATUS);
    if status == RUNNING {
        return None;
    }

    let text: String = (TEXT..TEXT + TEXT_LIMIT)
        .map(|address| emu.peek_at(address))
        .take_while(|&byte| byte != 0)
        .map(|byte| byte as char)
        .collect();

    Some(if status == PASSED {
        Outcome::Passed
    } else {
        Outcome::Failed(format!("code {status:#04X}: {}", text.trim()))
    })
}

/// Runs a mooneye ROM until it hits its `LD B,B` breakpoint.
///
/// `LD B,B` is a two-byte no-op that no compiler emits and mooneye reserves as
/// its stop signal, which is what makes it usable as one. The register file at
/// that instant is the entire result protocol.
pub fn mooneye(rom: &Path, budget_secs: u64) -> Run {
    /// The first six Fibonacci numbers, in B C D E H L. Chosen by mooneye
    /// because a half-working emulator is unlikely to land on them by accident.
    const PASS: [u8; 6] = [3, 5, 8, 13, 21, 34];
    /// Every register set to this means the ROM decided it failed.
    const FAIL: u8 = 0x42;

    // Every instruction: the breakpoint is one opcode wide, and stepping past it
    // loses the register file that is the entire result.
    drive(rom, budget_secs, 1, |emu, _| {
        // 0x40 is `LD B,B`.
        if emu.peek_opcode() != 0x40 {
            return None;
        }
        let r = emu.regs();
        let actual = [r.b, r.c, r.d, r.e, r.h, r.l];
        Some(if actual == PASS {
            Outcome::Passed
        } else if actual == [FAIL; 6] {
            Outcome::Failed("registers report failure".to_string())
        } else {
            Outcome::Failed(format!(
                "B:{:02X} C:{:02X} D:{:02X} E:{:02X} H:{:02X} L:{:02X}, want {:02X?}",
                r.b, r.c, r.d, r.e, r.h, r.l, PASS
            ))
        })
    })
}

/// An [`Output`] that keeps the last frame written to it.
///
/// Used by the snapshot tests: two runs of the same ROM for the same number of
/// cycles have to draw the same pixels, so a digest of this is a regression
/// test for the whole PPU at once.
pub struct Framebuffer {
    pixels: [[u8; SCREEN_WIDTH]; SCREEN_HEIGHT],
    /// Scanlines written, so a snapshot taken before the PPU ever ran is
    /// distinguishable from one of a legitimately blank screen.
    pub lines_written: u64,
}

impl Framebuffer {
    pub fn new() -> Self {
        Framebuffer { pixels: [[0; SCREEN_WIDTH]; SCREEN_HEIGHT], lines_written: 0 }
    }

    /// FNV-1a over the visible pixels, masked to what actually reaches an LCD.
    ///
    /// The sprite bit is deliberately excluded: it tells a backend where a pixel
    /// came from, not what shade it is, so including it would make the digest
    /// sensitive to changes that are invisible on hardware.
    pub fn digest(&self) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for row in self.pixels.iter() {
            for px in row.iter() {
                hash ^= (px & (PX_COLOR | PX_PALETTE)) as u64;
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        hash
    }

    /// The frame as ASCII, four shades to four characters. For eyeballing a
    /// snapshot that changed.
    pub fn to_ascii(&self) -> String {
        const SHADES: [char; 4] = ['#', '+', '.', ' '];
        let mut out = String::with_capacity((SCREEN_WIDTH + 1) * SCREEN_HEIGHT);
        for row in self.pixels.iter() {
            for px in row.iter() {
                out.push(SHADES[(px & PX_COLOR) as usize]);
            }
            out.push('\n');
        }
        out
    }
}

impl Default for Framebuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Output for Framebuffer {
    fn write_line(&mut self, y: u16, line: &[u8; SCREEN_WIDTH]) {
        if let Some(row) = self.pixels.get_mut(y as usize) {
            row.copy_from_slice(line);
            self.lines_written += 1;
        }
    }
}

/// Runs a ROM for exactly `cycles` M-cycles and returns the emulator, so the
/// caller can read back the framebuffer, the register file or the cycle count.
///
/// Deterministic by construction: there is no wall clock anywhere in the path,
/// the input backend reports no buttons, and the stop condition is emulated
/// time. Two runs of the same ROM must agree exactly, which is what makes a
/// digest of the result a usable snapshot.
pub fn run_for_cycles(rom: &Path, cycles: u64) -> Emulator<input::Dummy, Framebuffer> {
    let mut emu = Emulator::new(
        Rom::file(rom.to_str().expect("rom path is not utf-8")),
        input::Dummy::new(),
        Framebuffer::new(),
    );
    let mut serial = String::new();
    while emu.cycles() < cycles {
        emu.step(&mut serial);
    }
    emu
}

/// Where the vendored blargg suite lives, relative to the crate root -- which
/// is the working directory cargo gives a test, a bench and an example alike.
pub const BLARGG_ROOT: &str = "test-roms/gb-test-roms-master";
/// Where the vendored mooneye suite lives.
pub const MOONEYE_ROOT: &str = "test-roms/mooneye-test-suite";

/// The blargg ROMs this repository runs, relative to [`BLARGG_ROOT`].
///
/// `cgb_sound` is left out: it is a Game Boy Color suite and this is a DMG, so
/// its result would say nothing about a regression here.
pub fn blargg_roms() -> Vec<PathBuf> {
    roms_under(Path::new(BLARGG_ROOT))
        .into_iter()
        .filter(|rom| !rom.starts_with("cgb_sound"))
        .collect()
}

/// The mooneye ROMs this repository runs, relative to [`MOONEYE_ROOT`].
///
/// Two directories are left out because they are not pass/fail tests at all:
/// `utils` are cartridge dumpers meant to run on real hardware, and
/// `manual-only` has to be judged by eye against a reference photograph.
pub fn mooneye_roms() -> Vec<PathBuf> {
    roms_under(Path::new(MOONEYE_ROOT))
        .into_iter()
        .filter(|rom| !rom.starts_with("utils") && !rom.starts_with("manual-only"))
        .collect()
}

/// A suite-relative ROM path as it appears in the test declarations: forward
/// slashes on every platform, so the same literal works on Windows.
pub fn rom_name(rom: &Path) -> String {
    rom.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Every `.gb` under `dir`, sorted, as paths relative to `dir`.
///
/// Sorted so that a generated list of tests is stable across machines --
/// directory order is not.
pub fn roms_under(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        let entries = match std::fs::read_dir(&next) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "gb") {
                found.push(path.strip_prefix(dir).unwrap_or(&path).to_path_buf());
            }
        }
    }
    found.sort();
    found
}

/// Formats a report line the way the survey tool and a failing test both want
/// it: outcome, emulated time, wall-clock time.
pub fn summarise(name: &str, run: &Run) -> String {
    let mut line = String::new();
    let _ = write!(
        line,
        "{:<52} {:<9} {:>7.2}s emu  {:>7.1}ms wall",
        name,
        match &run.outcome {
            Outcome::Passed => "pass",
            Outcome::Failed(_) => "FAIL",
            Outcome::TimedOut => "timeout",
        },
        run.cycles as f64 / CYCLES_PER_SECOND as f64,
        run.elapsed.as_secs_f64() * 1000.0,
    );
    line
}
