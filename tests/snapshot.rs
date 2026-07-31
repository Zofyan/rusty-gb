//! Whole-machine snapshots: run a ROM for an exact number of cycles, then check
//! that everything about the machine is where it was last time.
//!
//! The two ROM suites next door check the emulator against real hardware, which
//! means most of them are ignored -- this emulator is not cycle-stepped and
//! knows it. These check the emulator against *itself*, so they apply to every
//! part of it, accurate or not. That is the gap they fill: an optimisation to
//! the PPU that quietly shifts a scanline, or a refactor that drops a cycle off
//! one opcode, changes nothing blargg tests but changes these immediately.
//!
//! Each snapshot pins:
//!
//! * the M-cycle count, which is what DIV, TIMA and the PPU are all derived
//!   from, so it moving means emulated time itself moved;
//! * a digest of the 160x144 framebuffer, i.e. the entire rendering path;
//! * the scanline count and the register file.
//!
//! Determinism is what makes this legitimate: with the dummy input backend and
//! a cycle count as the stop condition, there is no wall clock and no entropy
//! anywhere in the path. [`is_reproducible`] is the test that says so, and it
//! is the one to look at first if these start flapping.
//!
//! # When one of these fails
//!
//! A failure is not automatically a bug -- it says behaviour changed, not that
//! it got worse. Check the ROM suites first: if they are unchanged or better,
//! the new numbers are the correct ones and the fix is to update the literal
//! below. `assert_eq!` prints the observed [`State`] in the same shape as the
//! table, so updating it is a copy of the left-hand side.

use rusty_gb::testrom::{self, Framebuffer, CYCLES_PER_SECOND};
use std::path::{Path, PathBuf};

/// Everything a snapshot pins down.
///
/// Register pairs rather than the eight separate registers, because that is how
/// the Game Boy's own documentation names them and how a divergence reads.
#[derive(Debug, PartialEq, Eq)]
struct State {
    /// M-cycles actually executed. Not equal to the budget: the run stops on
    /// the first instruction to reach it, and instructions are 1-6 cycles.
    cycles: u64,
    /// FNV-1a over the visible framebuffer. See [`Framebuffer::digest`].
    frame: u64,
    /// Scanlines handed to the output backend.
    lines: u64,
    af: u16,
    bc: u16,
    de: u16,
    hl: u16,
    sp: u16,
    pc: u16,
}

fn rom_path(rom: &str) -> PathBuf {
    Path::new("test-roms").join(rom)
}

/// Runs `rom` for `secs` emulated seconds and reads the machine back.
fn observe(rom: &str, secs: u64) -> State {
    let emu = testrom::run_for_cycles(&rom_path(rom), secs * CYCLES_PER_SECOND);
    let r = emu.regs();
    State {
        cycles: emu.cycles(),
        frame: emu.output().digest(),
        lines: emu.output().lines_written,
        af: (r.a as u16) << 8 | r.f as u16,
        bc: (r.b as u16) << 8 | r.c as u16,
        de: (r.d as u16) << 8 | r.e as u16,
        hl: (r.h as u16) << 8 | r.l as u16,
        sp: r.sp,
        pc: r.pc,
    }
}

macro_rules! snapshots {
    ($( $name:ident, $rom:literal, $secs:literal, $want:expr ; )*) => {
        $(
            #[test]
            fn $name() {
                let want = $want;
                assert_eq!(observe($rom, $secs), want, "snapshot moved for {}", $rom);

                // A framebuffer of nothing but shade 0 has a perfectly stable
                // digest, so a snapshot taken before the ROM drew anything would
                // pin the PPU without exercising it. Every entry here is placed
                // past that point on purpose; this is what keeps it that way.
                assert_ne!(
                    want.frame,
                    Framebuffer::new().digest(),
                    "{} draws nothing by {}s -- move the snapshot later",
                    $rom, $secs
                );
            }
        )*

        /// Every ROM under snapshot, for [`is_reproducible`].
        const SNAPSHOTTED: &[(&str, u64)] = &[ $(($rom, $secs)),* ];
    };
}

snapshots! {
    cpu_instrs, "gb-test-roms-master/cpu_instrs/cpu_instrs.gb", 5, State {
        cycles: 5242882, frame: 0x954E5B30984571A0, lines: 43000,
        af: 0xF400, bc: 0x5691, de: 0x000E, hl: 0xDB90, sp: 0xDF6E, pc: 0xC5F3,
    };
    cpu_instrs_01_special, "gb-test-roms-master/cpu_instrs/individual/01-special.gb", 1, State {
        cycles: 1048576, frame: 0xB111D771B49F8B56, lines: 8598,
        af: 0x7F00, bc: 0xB1C7, de: 0x1F58, hl: 0x0180, sp: 0xDFF1, pc: 0xC07B,
    };
    instr_timing, "gb-test-roms-master/instr_timing/instr_timing.gb", 1, State {
        cycles: 1048576, frame: 0xEF5E88F08B198A44, lines: 8598,
        af: 0x00C0, bc: 0xD826, de: 0xD826, hl: 0xCC00, sp: 0xDFFF, pc: 0xC8B0,
    };
    oam_bug_2_causes, "gb-test-roms-master/oam_bug/rom_singles/2-causes.gb", 3, State {
        cycles: 3145729, frame: 0x329C70321015DDC6, lines: 25804,
        af: 0x0040, bc: 0x00FF, de: 0xFF00, hl: 0xFE00, sp: 0xE000, pc: 0xC9D3,
    };
    dmg_sound, "gb-test-roms-master/dmg_sound/dmg_sound.gb", 1, State {
        cycles: 1048578, frame: 0x0CB370AD3CE61F6B, lines: 8598,
        af: 0x4940, bc: 0x0000, de: 0xD000, hl: 0xFFFF, sp: 0xDFF2, pc: 0xC78C,
    };
    halt_bug, "gb-test-roms-master/halt_bug.gb", 2, State {
        cycles: 2097154, frame: 0x425BEAC55896766E, lines: 17196,
        af: 0x00C0, bc: 0x4D6F, de: 0x9F59, hl: 0xC21D, sp: 0xE000, pc: 0xC818,
    };
    mbc1_bits_bank1, "mooneye-test-suite/emulator-only/mbc1/bits_bank1.gb", 3, State {
        cycles: 3145728, frame: 0x8D764179B8C54A4A, lines: 25804,
        af: 0x00D0, bc: 0x0305, de: 0x080D, hl: 0x1522, sp: 0xE000, pc: 0x4879,
    };
    ppu_intr_1_2_timing, "mooneye-test-suite/acceptance/ppu/intr_1_2_timing-GS.gb", 1, State {
        cycles: 1048578, frame: 0x0275C3DB1EAA79A4, lines: 8598,
        af: 0x00D0, bc: 0x0305, de: 0x080D, hl: 0x1522, sp: 0xE000, pc: 0x4AB4,
    };
}

/// The same run twice gives the same machine.
///
/// Everything above assumes this. If it ever fails, the snapshots are not
/// wrong -- something has introduced a dependency on the host into the emulator
/// core, and the wall clock behind `platform::secs` (which the MBC3 RTC reads)
/// is the first place to look.
#[test]
fn is_reproducible() {
    for (rom, secs) in SNAPSHOTTED {
        assert_eq!(observe(rom, *secs), observe(rom, *secs), "{rom} is not deterministic");
    }
}

/// Running longer is running further.
///
/// Cheap, but it catches a whole class of mistake the digests cannot: a stop
/// condition that overshoots, or a cycle counter that saturates or resets. The
/// snapshots would happily pin a wrong-but-stable number.
#[test]
fn cycles_advance_monotonically() {
    let rom = rom_path("gb-test-roms-master/cpu_instrs/individual/01-special.gb");
    let mut previous = 0;
    for budget in [1u64, 1000, 100_000, 1_000_000] {
        let emu = testrom::run_for_cycles(&rom, budget);
        assert!(emu.cycles() >= budget, "stopped at {} short of {budget}", emu.cycles());
        assert!(emu.cycles() > previous, "{} did not advance past {previous}", emu.cycles());
        // One instruction is at most six M-cycles, so a run can only overshoot
        // its budget by five.
        assert!(emu.cycles() - budget < 6, "overshot {budget} by {}", emu.cycles() - budget);
        previous = emu.cycles();
    }
}
