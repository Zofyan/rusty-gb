use crate::bus::Bus;
use crate::cpu::Cpu;
use crate::input::Input;
use crate::output::Output;
use crate::ppu::{Ppu};
use bitfield::Bit;
use alloc::format;
use core::fmt::Write;
use crate::platform;
use crate::rom::Rom;

/// Interrupt vectors, indexed by IF/IE bit: vblank, LCD, timer, serial, joypad.
const INT_VECTORS: [u16; 5] = [0x40, 0x48, 0x50, 0x58, 0x60];

/// Frames between live FPS reports. Cheap enough to lower further: the port
/// carries a few lines a second against an endpoint good for tens of KB/s, and
/// the arithmetic below is integer.
const FPS_REPORT_INTERVAL: u32 = 15;

/// Microseconds a frame gets at the Game Boy's ~59.7 Hz.
const FRAME_BUDGET_US: u64 = 1_000_000 / 60;

/// Frame rate in tenths of a frame per second: `frames` frames spanning `us`
/// microseconds.
///
/// Integer throughout. `f64` is soft-float on the M33 -- its FPU is
/// single-precision -- and formatting one drags in `flt2dec`'s Grisu and Dragon
/// tables, which is a lot of flash for a diagnostic. Tenths give one decimal
/// place, which is all the readout ever showed.
///
/// Rounded rather than truncated: a frame that takes exactly its 16667 us budget
/// floors to 599 tenths, which would report a steady 60 fps as "59.9".
#[inline]
fn fps_tenths(frames: u64, us: u64) -> u64 {
    let us = us.max(1);
    (frames * 10_000_000 + us / 2) / us
}

pub struct Emulator<I: Input, O: Output> {
    cpu: Cpu,
    bus: Bus,
    ppu: Ppu,
    output: O,
    input: I,
    /// Elapsed time across `frames`, in microseconds.
    ///
    /// Accumulating time rather than a sum of per-frame rates is both cheaper --
    /// two integer adds per frame, no division until a report is due -- and more
    /// correct: the mean of per-frame rates is not the average frame rate, and
    /// the old sum-of-rates skewed the average towards the fastest frames.
    ///
    /// Kept as a running total rather than a Vec of per-frame samples: the Vec
    /// grew unbounded (~96 KB over a 12000-frame run) for no benefit.
    micros_total: u64,
    frames: u32,
}

impl<I: Input, O: Output> Emulator<I, O> {
    pub fn new(game: Rom, input: I, output: O) -> Self {

        let mut bus = Bus::new(game);
        let cpu = Cpu::new();
        let ppu = Ppu::new();

        bus.load_rom();

        bus.set_int_enable_lcd(true);
        bus.set_int_enable_joypad(true);
        bus.set_int_enable_serial(true);
        bus.set_int_enable_vblank(true);
        bus.set_int_enable_timer(true);

        bus.set_int_request_lcd(false);
        bus.set_int_request_joypad(false);
        bus.set_int_request_serial(false);
        bus.set_int_request_vblank(false);
        bus.set_int_request_timer(false);

        Emulator {
            cpu,
            bus,
            ppu,
            output,
            input,
            micros_total: 0,
            frames: 0,
        }
    }

    /// Executed from SRAM on the RP2350 rather than from flash.
    ///
    /// Code fetched over XIP goes through a 16 KiB cache; the interpreter's
    /// working set is larger than that, so leaving the hot path in flash costs a
    /// QSPI transaction on every miss. `.data.*` is collected into `.data`,
    /// which cortex-m-rt copies to RAM at startup. Cold code (the mappers, the
    /// RTC) is deliberately left in flash so it does not consume SRAM.
    ///
    /// `serial` receives the Game Boy's serial port verbatim and nothing else.
    /// Diagnostics go to `diag` instead, because they are emitted between frames
    /// while the ROM writes SB a byte at a time: sharing one sink let an FPS
    /// line land mid-token and split a test ROM's "Passed" in half. The two may
    /// still be the same underlying device -- on the Pico they are both the CDC
    /// port -- but they are separate sinks so a consumer that parses `serial`
    /// sees a clean stream.
    #[cfg_attr(target_os = "none", link_section = ".data.ram_func")]
    pub fn run(&mut self, max_cycles: usize, serial: &mut dyn Write, diag: &mut dyn Write) {
        let mut count: usize = 0;
        let mut timer: u64 = 0;
        loop {
            let millis = platform::micros();
            for i in 0..17476 {
                self.input.check_input(&mut self.bus);
                // Interrupts are five enable/request bit pairs; testing them as a
                // single mask replaces ten bit extractions per instruction with
                // one AND. Bit order (vblank..joypad) matches the vector order,
                // so the lowest set bit is the highest-priority interrupt.
                let pending = self.bus.pending_interrupts();
                let cycles = if self.cpu.get_ime() && pending != 0 {
                    let bit = pending.trailing_zeros() as u8;
                    self.bus.clear_int_request(bit);
                    self.cpu.interrupt(&mut self.bus, INT_VECTORS[bit as usize])
                } else {
                    self.cpu.step(&mut self.bus, true)
                };

                let previous_timer = timer;
                timer += cycles as u64;

                self.ppu.tick(&mut self.bus, &mut self.output, cycles);

                // DIV ticks once per 64 M-cycles and TIMA once per TAC period.
                // Both are powers of two, so the number of ticks elapsed over
                // this instruction is a shift difference -- no loop, no modulo.
                let div_ticks = (timer >> 6) - (previous_timer >> 6);
                if div_ticks != 0 {
                    self.bus.registers.div = self.bus.registers.div.wrapping_add(div_ticks as u8);
                }

                if self.bus.registers.tca.bit(2) {
                    let shift = match self.bus.registers.tca & 0x3 {
                        0 => 8, // 256 M-cycles
                        1 => 2, // 4
                        2 => 4, // 16
                        3 => 6, // 64
                        _ => unreachable!(),
                    };
                    let tima_ticks = (timer >> shift) - (previous_timer >> shift);
                    for _ in 0..tima_ticks {
                        let val = self.bus.registers.tima.wrapping_add(1);
                        self.bus.registers.tima = val;
                        if val == 0 {
                            self.bus.registers.tima = self.bus.registers.tma;
                            self.bus.set_int_request_timer(true);
                        }
                    }
                }

                if self.bus.registers.sc == 0x81 {
                    // `core::fmt::Write` has no `flush`; the serial sink in
                    // `main` writes through, so there is nothing to drain.
                    write!(serial, "{}", self.bus.registers.sb as char).expect("Couldn't write");
                    self.bus.registers.sc = 0;
                }
            }
            count += 1;
            if count > max_cycles && max_cycles != 0 {
                let avg = fps_tenths(self.frames as u64, self.micros_total);
                let _ = writeln!(diag, "Avg FPS: {}.{}", avg / 10, avg % 10);
                break;
            }
            if !self.output.refresh() {
                break;
            }

            // `max(1)` guards the division below: a frame can finish inside one
            // tick of the microsecond counter.
            let diff = (platform::micros() - millis).max(1);
            self.micros_total += diff;
            self.frames += 1;

            if diff < FRAME_BUDGET_US {
                //sleep(Duration::from_micros(FRAME_BUDGET_US - diff))
            }

            let inst = fps_tenths(1, diff);

            // Live readout. Emitted between frames, so it cannot interrupt a
            // partially written line of the ROM's own serial output.
            if self.frames % FPS_REPORT_INTERVAL == 0 {
                let avg = fps_tenths(self.frames as u64, self.micros_total);
                let _ = writeln!(
                    diag,
                    "FPS: {}.{} (avg {}.{})",
                    inst / 10,
                    inst % 10,
                    avg / 10,
                    avg % 10
                );
            }
            self.output.set_diagnostics(format!("FPS: {}.{}", inst / 10, inst % 10));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{fps_tenths, FRAME_BUDGET_US};

    #[test]
    fn fps_tenths_reports_one_decimal_place() {
        // A frame that lands exactly on budget is 60.0, not 59.9.
        assert_eq!(fps_tenths(1, FRAME_BUDGET_US), 600);
        // Half a budget is double the rate.
        assert_eq!(fps_tenths(1, FRAME_BUDGET_US / 2), 1200);
        // An average is frames over total time, not a mean of rates.
        assert_eq!(fps_tenths(90, 3_000_000), 300);
        assert_eq!(fps_tenths(1, 400_000), 25);
    }

    /// A frame can complete inside one tick of the microsecond counter, and the
    /// caller clamps to 1; `fps_tenths` must not divide by zero even if it does
    /// not.
    #[test]
    fn fps_tenths_survives_a_zero_interval() {
        assert_eq!(fps_tenths(1, 0), 10_000_000);
    }
}
