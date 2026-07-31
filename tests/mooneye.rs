//! Gekkio's mooneye test suite, one `#[test]` per ROM.
//!
//! Where blargg's ROMs check that the emulator computes the right answer,
//! mooneye's check that it computes it at the right *moment*: most of
//! `acceptance/` measures an instruction's observable effect against a timer
//! ticking underneath it. That makes this suite the harder of the two by a wide
//! margin, and it is the one whose score moves when the emulator's timing model
//! changes.
//!
//! The protocol is a `LD B,B` breakpoint with the verdict in the register file;
//! [`rusty_gb::testrom::mooneye`] has the details.
//!
//! # The ignored ones
//!
//! 95 of 112 are ignored, which is the honest state of a per-instruction
//! interpreter against a suite written for a cycle-stepped one. The reasons
//! group into a handful of causes -- no boot ROM, no per-dot PPU, no
//! per-M-cycle timer, no MBC5 -- and each ROM carries the one that applies.
//! They are here rather than deleted because they are the roadmap: `cargo test
//! --test mooneye -- --ignored` measures progress on any of those fronts.
//!
//! Some of the ignored ones panic rather than fail, where the ROM needs a
//! mapper the emulator does not implement. That is why they cannot simply be
//! left un-ignored.

use rusty_gb::testrom::{self, BUDGET_MOONEYE_SECS};
use std::path::Path;

/// Declares one `#[test]` per ROM, plus the completeness check over all of them.
macro_rules! mooneye_suite {
    ($( $name:ident, $rom:literal $(, $ignore:literal)? ; )*) => {
        $(
            $( #[ignore = $ignore] )?
            #[test]
            fn $name() {
                testrom::mooneye(&Path::new(testrom::MOONEYE_ROOT).join($rom), BUDGET_MOONEYE_SECS)
                    .assert_passed();
            }
        )*

        /// Every ROM with a test above.
        const DECLARED: &[&str] = &[ $($rom),* ];
    };
}

mooneye_suite! {
    acceptance_add_sp_e_timing, "acceptance/add_sp_e_timing.gb", "instruction timing is not cycle-stepped (timeout)";
    acceptance_bits_mem_oam, "acceptance/bits/mem_oam.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_bits_reg_f, "acceptance/bits/reg_f.gb";
    acceptance_bits_unused_hwio_gs, "acceptance/bits/unused_hwio-GS.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_boot_div_s, "acceptance/boot_div-S.gb", "no boot ROM: post-boot state is only approximated (fail)";
    acceptance_boot_div_dmg0, "acceptance/boot_div-dmg0.gb", "no boot ROM: post-boot state is only approximated (fail)";
    acceptance_boot_div_dmgabcmgb, "acceptance/boot_div-dmgABCmgb.gb", "no boot ROM: post-boot state is only approximated (fail)";
    acceptance_boot_div2_s, "acceptance/boot_div2-S.gb", "no boot ROM: post-boot state is only approximated (fail)";
    acceptance_boot_hwio_s, "acceptance/boot_hwio-S.gb", "no boot ROM: post-boot state is only approximated (fail)";
    acceptance_boot_hwio_dmg0, "acceptance/boot_hwio-dmg0.gb", "no boot ROM: post-boot state is only approximated (fail)";
    acceptance_boot_hwio_dmgabcmgb, "acceptance/boot_hwio-dmgABCmgb.gb", "no boot ROM: post-boot state is only approximated (fail)";
    acceptance_boot_regs_dmg0, "acceptance/boot_regs-dmg0.gb", "no boot ROM: post-boot state is only approximated (fail)";
    acceptance_boot_regs_dmgabc, "acceptance/boot_regs-dmgABC.gb";
    acceptance_boot_regs_mgb, "acceptance/boot_regs-mgb.gb", "no boot ROM: post-boot state is only approximated (fail)";
    acceptance_boot_regs_sgb, "acceptance/boot_regs-sgb.gb", "no boot ROM: post-boot state is only approximated (fail)";
    acceptance_boot_regs_sgb2, "acceptance/boot_regs-sgb2.gb", "no boot ROM: post-boot state is only approximated (fail)";
    acceptance_call_cc_timing, "acceptance/call_cc_timing.gb", "instruction timing is not cycle-stepped (timeout)";
    acceptance_call_cc_timing2, "acceptance/call_cc_timing2.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_call_timing, "acceptance/call_timing.gb", "instruction timing is not cycle-stepped (timeout)";
    acceptance_call_timing2, "acceptance/call_timing2.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_di_timing_gs, "acceptance/di_timing-GS.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_div_timing, "acceptance/div_timing.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_ei_sequence, "acceptance/ei_sequence.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_ei_timing, "acceptance/ei_timing.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_halt_ime0_ei, "acceptance/halt_ime0_ei.gb";
    acceptance_halt_ime0_nointr_timing, "acceptance/halt_ime0_nointr_timing.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_halt_ime1_timing, "acceptance/halt_ime1_timing.gb";
    acceptance_halt_ime1_timing2_gs, "acceptance/halt_ime1_timing2-GS.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_if_ie_registers, "acceptance/if_ie_registers.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_instr_daa, "acceptance/instr/daa.gb";
    acceptance_interrupts_ie_push, "acceptance/interrupts/ie_push.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_intr_timing, "acceptance/intr_timing.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_jp_cc_timing, "acceptance/jp_cc_timing.gb", "instruction timing is not cycle-stepped (timeout)";
    acceptance_jp_timing, "acceptance/jp_timing.gb", "instruction timing is not cycle-stepped (timeout)";
    acceptance_ld_hl_sp_e_timing, "acceptance/ld_hl_sp_e_timing.gb", "instruction timing is not cycle-stepped (timeout)";
    acceptance_oam_dma_basic, "acceptance/oam_dma/basic.gb", "OAM DMA is not cycle-stepped (fail)";
    acceptance_oam_dma_reg_read, "acceptance/oam_dma/reg_read.gb", "OAM DMA is not cycle-stepped (fail)";
    acceptance_oam_dma_sources_gs, "acceptance/oam_dma/sources-GS.gb", "OAM DMA is not cycle-stepped (crashed)";
    acceptance_oam_dma_restart, "acceptance/oam_dma_restart.gb", "OAM DMA is not cycle-stepped (fail)";
    acceptance_oam_dma_start, "acceptance/oam_dma_start.gb", "OAM DMA is not cycle-stepped (fail)";
    acceptance_oam_dma_timing, "acceptance/oam_dma_timing.gb", "OAM DMA is not cycle-stepped (fail)";
    acceptance_pop_timing, "acceptance/pop_timing.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_ppu_hblank_ly_scx_timing_gs, "acceptance/ppu/hblank_ly_scx_timing-GS.gb", "PPU is stepped per instruction, not per dot (fail)";
    acceptance_ppu_intr_1_2_timing_gs, "acceptance/ppu/intr_1_2_timing-GS.gb";
    acceptance_ppu_intr_2_0_timing, "acceptance/ppu/intr_2_0_timing.gb", "PPU is stepped per instruction, not per dot (timeout)";
    acceptance_ppu_intr_2_mode0_timing, "acceptance/ppu/intr_2_mode0_timing.gb", "PPU is stepped per instruction, not per dot (timeout)";
    acceptance_ppu_intr_2_mode0_timing_sprites, "acceptance/ppu/intr_2_mode0_timing_sprites.gb", "PPU is stepped per instruction, not per dot (timeout)";
    acceptance_ppu_intr_2_mode3_timing, "acceptance/ppu/intr_2_mode3_timing.gb", "PPU is stepped per instruction, not per dot (timeout)";
    acceptance_ppu_intr_2_oam_ok_timing, "acceptance/ppu/intr_2_oam_ok_timing.gb", "PPU is stepped per instruction, not per dot (timeout)";
    acceptance_ppu_lcdon_timing_gs, "acceptance/ppu/lcdon_timing-GS.gb", "PPU is stepped per instruction, not per dot (fail)";
    acceptance_ppu_lcdon_write_timing_gs, "acceptance/ppu/lcdon_write_timing-GS.gb", "PPU is stepped per instruction, not per dot (fail)";
    acceptance_ppu_stat_irq_blocking, "acceptance/ppu/stat_irq_blocking.gb", "PPU is stepped per instruction, not per dot (timeout)";
    acceptance_ppu_stat_lyc_onoff, "acceptance/ppu/stat_lyc_onoff.gb", "PPU is stepped per instruction, not per dot (fail)";
    acceptance_ppu_vblank_stat_intr_gs, "acceptance/ppu/vblank_stat_intr-GS.gb", "PPU is stepped per instruction, not per dot (fail)";
    acceptance_push_timing, "acceptance/push_timing.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_rapid_di_ei, "acceptance/rapid_di_ei.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_ret_cc_timing, "acceptance/ret_cc_timing.gb", "instruction timing is not cycle-stepped (timeout)";
    acceptance_ret_timing, "acceptance/ret_timing.gb", "instruction timing is not cycle-stepped (timeout)";
    acceptance_reti_intr_timing, "acceptance/reti_intr_timing.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_reti_timing, "acceptance/reti_timing.gb", "instruction timing is not cycle-stepped (timeout)";
    acceptance_rst_timing, "acceptance/rst_timing.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_serial_boot_sclk_align_dmgabcmgb, "acceptance/serial/boot_sclk_align-dmgABCmgb.gb", "instruction timing is not cycle-stepped (fail)";
    acceptance_timer_div_write, "acceptance/timer/div_write.gb", "timer is instruction-granular (fail)";
    acceptance_timer_rapid_toggle, "acceptance/timer/rapid_toggle.gb", "timer is instruction-granular (fail)";
    acceptance_timer_tim00, "acceptance/timer/tim00.gb", "timer is instruction-granular (fail)";
    acceptance_timer_tim00_div_trigger, "acceptance/timer/tim00_div_trigger.gb";
    acceptance_timer_tim01, "acceptance/timer/tim01.gb", "timer is instruction-granular (fail)";
    acceptance_timer_tim01_div_trigger, "acceptance/timer/tim01_div_trigger.gb", "timer is instruction-granular (fail)";
    acceptance_timer_tim10, "acceptance/timer/tim10.gb", "timer is instruction-granular (fail)";
    acceptance_timer_tim10_div_trigger, "acceptance/timer/tim10_div_trigger.gb", "timer is instruction-granular (fail)";
    acceptance_timer_tim11, "acceptance/timer/tim11.gb", "timer is instruction-granular (fail)";
    acceptance_timer_tim11_div_trigger, "acceptance/timer/tim11_div_trigger.gb", "timer is instruction-granular (fail)";
    acceptance_timer_tima_reload, "acceptance/timer/tima_reload.gb", "timer is instruction-granular (fail)";
    acceptance_timer_tima_write_reloading, "acceptance/timer/tima_write_reloading.gb", "timer is instruction-granular (fail)";
    acceptance_timer_tma_write_reloading, "acceptance/timer/tma_write_reloading.gb", "timer is instruction-granular (fail)";
    emulator_only_mbc1_bits_bank1, "emulator-only/mbc1/bits_bank1.gb";
    emulator_only_mbc1_bits_bank2, "emulator-only/mbc1/bits_bank2.gb";
    emulator_only_mbc1_bits_mode, "emulator-only/mbc1/bits_mode.gb", "mapper edge cases (fail)";
    emulator_only_mbc1_bits_ramg, "emulator-only/mbc1/bits_ramg.gb", "mapper edge cases (fail)";
    emulator_only_mbc1_multicart_rom_8mb, "emulator-only/mbc1/multicart_rom_8Mb.gb", "mapper edge cases (timeout)";
    emulator_only_mbc1_ram_256kb, "emulator-only/mbc1/ram_256kb.gb", "mapper edge cases (fail)";
    emulator_only_mbc1_ram_64kb, "emulator-only/mbc1/ram_64kb.gb", "mapper edge cases (fail)";
    emulator_only_mbc1_rom_16mb, "emulator-only/mbc1/rom_16Mb.gb", "mapper edge cases (crashed)";
    emulator_only_mbc1_rom_1mb, "emulator-only/mbc1/rom_1Mb.gb";
    emulator_only_mbc1_rom_2mb, "emulator-only/mbc1/rom_2Mb.gb";
    emulator_only_mbc1_rom_4mb, "emulator-only/mbc1/rom_4Mb.gb";
    emulator_only_mbc1_rom_512kb, "emulator-only/mbc1/rom_512kb.gb";
    emulator_only_mbc1_rom_8mb, "emulator-only/mbc1/rom_8Mb.gb", "mapper edge cases (timeout)";
    emulator_only_mbc2_bits_ramg, "emulator-only/mbc2/bits_ramg.gb", "mapper edge cases (crashed)";
    emulator_only_mbc2_bits_romb, "emulator-only/mbc2/bits_romb.gb";
    emulator_only_mbc2_bits_unused, "emulator-only/mbc2/bits_unused.gb", "mapper edge cases (crashed)";
    emulator_only_mbc2_ram, "emulator-only/mbc2/ram.gb", "mapper edge cases (crashed)";
    emulator_only_mbc2_rom_1mb, "emulator-only/mbc2/rom_1Mb.gb";
    emulator_only_mbc2_rom_2mb, "emulator-only/mbc2/rom_2Mb.gb";
    emulator_only_mbc2_rom_512kb, "emulator-only/mbc2/rom_512kb.gb";
    emulator_only_mbc5_rom_16mb, "emulator-only/mbc5/rom_16Mb.gb", "MBC5 not implemented (crashed)";
    emulator_only_mbc5_rom_1mb, "emulator-only/mbc5/rom_1Mb.gb", "MBC5 not implemented (crashed)";
    emulator_only_mbc5_rom_2mb, "emulator-only/mbc5/rom_2Mb.gb", "MBC5 not implemented (crashed)";
    emulator_only_mbc5_rom_32mb, "emulator-only/mbc5/rom_32Mb.gb", "MBC5 not implemented (crashed)";
    emulator_only_mbc5_rom_4mb, "emulator-only/mbc5/rom_4Mb.gb", "MBC5 not implemented (crashed)";
    emulator_only_mbc5_rom_512kb, "emulator-only/mbc5/rom_512kb.gb", "MBC5 not implemented (crashed)";
    emulator_only_mbc5_rom_64mb, "emulator-only/mbc5/rom_64Mb.gb", "MBC5 not implemented (crashed)";
    emulator_only_mbc5_rom_8mb, "emulator-only/mbc5/rom_8Mb.gb", "MBC5 not implemented (crashed)";
    madness_mgb_oam_dma_halt_sprites, "madness/mgb_oam_dma_halt_sprites.gb", "needs cycle-exact OAM DMA and sprites (timeout)";
    misc_bits_unused_hwio_c, "misc/bits/unused_hwio-C.gb", "CGB/AGB hardware, this is a DMG (fail)";
    misc_boot_div_a, "misc/boot_div-A.gb", "no boot ROM: post-boot state is only approximated (fail)";
    misc_boot_div_cgb0, "misc/boot_div-cgb0.gb", "no boot ROM: post-boot state is only approximated (fail)";
    misc_boot_div_cgbabcde, "misc/boot_div-cgbABCDE.gb", "no boot ROM: post-boot state is only approximated (fail)";
    misc_boot_hwio_c, "misc/boot_hwio-C.gb", "no boot ROM: post-boot state is only approximated (fail)";
    misc_boot_regs_a, "misc/boot_regs-A.gb", "no boot ROM: post-boot state is only approximated (fail)";
    misc_boot_regs_cgb, "misc/boot_regs-cgb.gb", "no boot ROM: post-boot state is only approximated (fail)";
    misc_ppu_vblank_stat_intr_c, "misc/ppu/vblank_stat_intr-C.gb", "PPU is stepped per instruction, not per dot (fail)";
}

/// Every vendored ROM has a test.
///
/// Without this, dropping a new ROM into `test-roms/` and forgetting to declare
/// it leaves it silently untested -- which is the failure mode a vendored suite
/// is most prone to, because adding files is the easy half.
#[test]
fn every_vendored_rom_is_declared() {
    let vendored: Vec<String> = testrom::mooneye_roms().iter().map(|rom| testrom::rom_name(rom)).collect();
    let missing: Vec<&String> = vendored.iter().filter(|rom| !DECLARED.contains(&rom.as_str())).collect();
    assert!(missing.is_empty(), "vendored but not declared: {missing:#?}");

    let stale: Vec<&&str> = DECLARED.iter().filter(|rom| !vendored.contains(&rom.to_string())).collect();
    assert!(stale.is_empty(), "declared but not vendored: {stale:#?}");
}
