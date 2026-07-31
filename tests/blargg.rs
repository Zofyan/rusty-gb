//! blargg's test suite, one `#[test]` per ROM.
//!
//! Each ROM here is a self-checking program: it exercises one corner of the
//! hardware and reports a verdict through [`rusty_gb::testrom`], which knows the
//! two ways blargg's ROMs do that. A test failing means the emulator got a
//! documented answer wrong, and the panic message carries the ROM's own report,
//! which names the sub-test.
//!
//! # The ignored ones
//!
//! Roughly two thirds of the suite is ignored, each with the reason it cannot
//! pass today rather than a bare "known failure". Those reasons are almost all
//! one of two things: this emulator has no APU at all, and it advances the PPU,
//! the timer and the bus a whole instruction at a time rather than a cycle at a
//! time. Nothing in the ignore list is a mystery -- if one becomes a mystery,
//! that is itself the bug.
//!
//! `cargo test --test blargg -- --ignored` runs them, which is how you check
//! whether a change to timing moved the needle. `cargo run --release --example
//! test-report` regenerates this list; see that file for the workflow.
//!
//! Ignoring rather than tracking expectations is a deliberate trade: a ROM that
//! starts passing stays quietly ignored until someone re-runs the survey. The
//! `--ignored` run is what surfaces it.

use rusty_gb::testrom::{self, BUDGET_BLARGG_SECS};
use std::path::Path;

/// Declares one `#[test]` per ROM, plus the completeness check over all of them.
///
/// Taking the whole suite in one invocation is what lets [`DECLARED`] be built
/// from the same list that generates the tests, so the two cannot drift.
macro_rules! blargg_suite {
    ($( $name:ident, $rom:literal $(, $ignore:literal)? ; )*) => {
        $(
            $( #[ignore = $ignore] )?
            #[test]
            fn $name() {
                testrom::blargg(&Path::new(testrom::BLARGG_ROOT).join($rom), BUDGET_BLARGG_SECS)
                    .assert_passed();
            }
        )*

        /// Every ROM with a test above.
        const DECLARED: &[&str] = &[ $($rom),* ];
    };
}

blargg_suite! {
    cpu_instrs_cpu_instrs, "cpu_instrs/cpu_instrs.gb";
    cpu_instrs_individual_01_special, "cpu_instrs/individual/01-special.gb";
    cpu_instrs_individual_02_interrupts, "cpu_instrs/individual/02-interrupts.gb";
    cpu_instrs_individual_03_op_sp_hl, "cpu_instrs/individual/03-op sp,hl.gb";
    cpu_instrs_individual_04_op_r_imm, "cpu_instrs/individual/04-op r,imm.gb";
    cpu_instrs_individual_05_op_rp, "cpu_instrs/individual/05-op rp.gb";
    cpu_instrs_individual_06_ld_r_r, "cpu_instrs/individual/06-ld r,r.gb";
    cpu_instrs_individual_07_jr_jp_call_ret_rst, "cpu_instrs/individual/07-jr,jp,call,ret,rst.gb";
    cpu_instrs_individual_08_misc_instrs, "cpu_instrs/individual/08-misc instrs.gb";
    cpu_instrs_individual_09_op_r_r, "cpu_instrs/individual/09-op r,r.gb";
    cpu_instrs_individual_10_bit_ops, "cpu_instrs/individual/10-bit ops.gb";
    cpu_instrs_individual_11_op_a_hl, "cpu_instrs/individual/11-op a,(hl).gb";
    dmg_sound_dmg_sound, "dmg_sound/dmg_sound.gb", "no APU (fail)";
    dmg_sound_rom_singles_01_registers, "dmg_sound/rom_singles/01-registers.gb", "no APU (fail)";
    dmg_sound_rom_singles_02_len_ctr, "dmg_sound/rom_singles/02-len ctr.gb", "no APU (fail)";
    dmg_sound_rom_singles_03_trigger, "dmg_sound/rom_singles/03-trigger.gb", "no APU (fail)";
    dmg_sound_rom_singles_04_sweep, "dmg_sound/rom_singles/04-sweep.gb", "no APU (fail)";
    dmg_sound_rom_singles_05_sweep_details, "dmg_sound/rom_singles/05-sweep details.gb", "no APU (fail)";
    dmg_sound_rom_singles_06_overflow_on_trigger, "dmg_sound/rom_singles/06-overflow on trigger.gb", "no APU (fail)";
    dmg_sound_rom_singles_07_len_sweep_period_sync, "dmg_sound/rom_singles/07-len sweep period sync.gb", "no APU (fail)";
    dmg_sound_rom_singles_08_len_ctr_during_power, "dmg_sound/rom_singles/08-len ctr during power.gb", "no APU (fail)";
    dmg_sound_rom_singles_09_wave_read_while_on, "dmg_sound/rom_singles/09-wave read while on.gb", "no APU (fail)";
    dmg_sound_rom_singles_10_wave_trigger_while_on, "dmg_sound/rom_singles/10-wave trigger while on.gb", "no APU (fail)";
    dmg_sound_rom_singles_11_regs_after_power, "dmg_sound/rom_singles/11-regs after power.gb", "no APU (fail)";
    dmg_sound_rom_singles_12_wave_write_while_on, "dmg_sound/rom_singles/12-wave write while on.gb", "no APU (fail)";
    halt_bug, "halt_bug.gb", "halt bug not emulated (fail)";
    instr_timing_instr_timing, "instr_timing/instr_timing.gb";
    interrupt_time_interrupt_time, "interrupt_time/interrupt_time.gb", "interrupt timing is instruction-granular (fail)";
    mem_timing_individual_01_read_timing, "mem_timing/individual/01-read_timing.gb", "memory timing is instruction-granular (fail)";
    mem_timing_individual_02_write_timing, "mem_timing/individual/02-write_timing.gb", "memory timing is instruction-granular (fail)";
    mem_timing_individual_03_modify_timing, "mem_timing/individual/03-modify_timing.gb", "memory timing is instruction-granular (fail)";
    mem_timing_mem_timing, "mem_timing/mem_timing.gb", "memory timing is instruction-granular (fail)";
    mem_timing_2_mem_timing, "mem_timing-2/mem_timing.gb", "memory timing is instruction-granular (fail)";
    mem_timing_2_rom_singles_01_read_timing, "mem_timing-2/rom_singles/01-read_timing.gb", "memory timing is instruction-granular (fail)";
    mem_timing_2_rom_singles_02_write_timing, "mem_timing-2/rom_singles/02-write_timing.gb", "memory timing is instruction-granular (fail)";
    mem_timing_2_rom_singles_03_modify_timing, "mem_timing-2/rom_singles/03-modify_timing.gb", "memory timing is instruction-granular (fail)";
    oam_bug_oam_bug, "oam_bug/oam_bug.gb", "OAM bug not emulated (fail)";
    oam_bug_rom_singles_1_lcd_sync, "oam_bug/rom_singles/1-lcd_sync.gb", "OAM bug not emulated (fail)";
    oam_bug_rom_singles_2_causes, "oam_bug/rom_singles/2-causes.gb";
    oam_bug_rom_singles_3_non_causes, "oam_bug/rom_singles/3-non_causes.gb", "OAM bug not emulated (fail)";
    oam_bug_rom_singles_4_scanline_timing, "oam_bug/rom_singles/4-scanline_timing.gb", "OAM bug not emulated (fail)";
    oam_bug_rom_singles_5_timing_bug, "oam_bug/rom_singles/5-timing_bug.gb";
    oam_bug_rom_singles_6_timing_no_bug, "oam_bug/rom_singles/6-timing_no_bug.gb", "OAM bug not emulated (fail)";
    oam_bug_rom_singles_7_timing_effect, "oam_bug/rom_singles/7-timing_effect.gb", "OAM bug not emulated (fail)";
    oam_bug_rom_singles_8_instr_effect, "oam_bug/rom_singles/8-instr_effect.gb", "OAM bug not emulated (fail)";
}

/// Every vendored ROM has a test.
///
/// Without this, dropping a new ROM into `test-roms/` and forgetting to declare
/// it leaves it silently untested -- which is the failure mode a vendored suite
/// is most prone to, because adding files is the easy half.
#[test]
fn every_vendored_rom_is_declared() {
    let vendored: Vec<String> = testrom::blargg_roms().iter().map(|rom| testrom::rom_name(rom)).collect();
    let missing: Vec<&String> = vendored.iter().filter(|rom| !DECLARED.contains(&rom.as_str())).collect();
    assert!(missing.is_empty(), "vendored but not declared: {missing:#?}");

    let stale: Vec<&&str> = DECLARED.iter().filter(|rom| !vendored.contains(&rom.to_string())).collect();
    assert!(stale.is_empty(), "declared but not vendored: {stale:#?}");
}
