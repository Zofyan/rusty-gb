use crate::bus::Bus;
use crate::cpu::{Cpu, Regs};
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

/// M-cycles in one real frame: 456 dots x 154 lines, four dots to the M-cycle.
///
/// The PPU derives its own timing from the cycles [`Emulator::step`] hands it,
/// so nothing in the emulator reads this. It is here for callers that want to
/// talk in frames -- the benchmarks, and anything measuring emulated time
/// against wall-clock time.
pub const CYCLES_PER_FRAME: u64 = 456 * 154 / 4;

/// Instructions [`Emulator::batch`] runs between [`Output::refresh`] calls.
///
/// Note that this counts *instructions*, not cycles: at a DMG average of
/// roughly three M-cycles an instruction, one batch covers something closer to
/// three frames' worth of emulated time than one. The frame rate `run` reports
/// is therefore refreshes per second, which is not the same number as emulated
/// frames per second -- `benches/throughput.rs` measures and prints both.
pub const STEPS_PER_BATCH: usize = 17476;

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
    /// Emulated M-cycles since power-on. DIV and TIMA are shift differences on
    /// this, so it has to be monotonic and it has to be exact.
    cycles: u64,
}
use crate::output::spi::CORE1_STATUS;
use core::sync::atomic::Ordering;
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
            cycles: 0,
        }
    }

    /// Runs one instruction, or services one pending interrupt, and returns the
    /// M-cycles it took.
    ///
    /// This is the whole emulator in one call: CPU, PPU, DIV/TIMA and the
    /// serial port all advance by exactly the cycles the instruction consumed.
    /// [`Emulator::batch`] is nothing but a loop over it, so a caller that needs
    /// to stop on a condition -- a test ROM's breakpoint, a cycle budget -- can
    /// drive this directly and see the same machine the binary runs.
    ///
    /// `#[inline(always)]` because `batch` is the hot loop on a core with no
    /// branch predictor: left out of line this would be a call and a return per
    /// instruction, inside the one function that was deliberately placed in
    /// SRAM to avoid exactly that kind of stall.
    #[inline(always)]
    pub fn step(&mut self, serial: &mut dyn Write) -> usize {
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

        let previous_timer = self.cycles;
        self.cycles += cycles as u64;

        self.ppu.tick(&mut self.bus, &mut self.output, cycles);

        // DIV ticks once per 64 M-cycles and TIMA once per TAC period.
        // Both are powers of two, so the number of ticks elapsed over
        // this instruction is a shift difference -- no loop, no modulo.
        let div_ticks = (self.cycles >> 6) - (previous_timer >> 6);
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
            let tima_ticks = (self.cycles >> shift) - (previous_timer >> shift);
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

        cycles
    }

    /// One unit of work between [`Output::refresh`] calls: [`STEPS_PER_BATCH`]
    /// instructions.
    ///
    /// Executed from SRAM on the RP2350 rather than from flash.
    ///
    /// Code fetched over XIP goes through a 16 KiB cache; the interpreter's
    /// working set is larger than that, so leaving the hot path in flash costs a
    /// QSPI transaction on every miss. `.data.*` is collected into `.data`,
    /// which cortex-m-rt copies to RAM at startup. Cold code (the mappers, the
    /// RTC) is deliberately left in flash so it does not consume SRAM. `run`
    /// itself stays in flash: it runs once per batch, not once per instruction.
    #[cfg_attr(target_os = "none", link_section = ".data.ram_func")]
    #[inline(never)]
    pub fn batch(&mut self, serial: &mut dyn Write) {
        for _ in 0..STEPS_PER_BATCH {
            self.step(serial);
        }
    }

    /// M-cycles executed since power-on.
    ///
    /// The same counter DIV and TIMA are derived from, so it is the emulator's
    /// own notion of time rather than a second one kept alongside it.
    pub fn cycles(&self) -> u64 {
        self.cycles
    }

    /// The CPU register file, for tests and debuggers.
    pub fn regs(&self) -> Regs {
        self.cpu.regs()
    }

    /// The opcode [`Emulator::step`] would execute next.
    ///
    /// Reading it costs a bus fetch, so this is for a caller stepping under a
    /// breakpoint condition, not for the hot loop.
    pub fn peek_opcode(&self) -> u8 {
        self.bus.get(self.cpu.regs().pc)
    }

    /// Whether the CPU has interrupts enabled. For inspecting a stopped
    /// machine: a test ROM waiting forever on an interrupt looks identical to
    /// one computing forever until you can see this.
    pub fn ime(&self) -> bool {
        self.cpu.ime()
    }

    /// Reads a bus address without side effects on the CPU. For debuggers and
    /// test harnesses inspecting a stopped machine.
    pub fn peek_at(&self, address: u16) -> u8 {
        self.bus.get(address)
    }

    /// The video sink, so a caller that supplied a capturing [`Output`] can read
    /// back what was drawn.
    pub fn output(&self) -> &O {
        &self.output
    }

    /// Runs until [`Output::refresh`] says to stop or `max_cycles` batches have
    /// gone by, reporting frame rate as it goes.
    ///
    /// `serial` receives the Game Boy's serial port verbatim and nothing else.
    /// Diagnostics go to `diag` instead, because they are emitted between frames
    /// while the ROM writes SB a byte at a time: sharing one sink let an FPS
    /// line land mid-token and split a test ROM's "Passed" in half. The two may
    /// still be the same underlying device -- on the Pico they are both the CDC
    /// port -- but they are separate sinks so a consumer that parses `serial`
    /// sees a clean stream.
    ///
    /// `max_cycles` counts calls to [`Emulator::batch`], not cycles and not
    /// frames; see that function for what one of them actually covers.
    pub fn run(&mut self, max_cycles: usize, serial: &mut dyn Write, diag: &mut dyn Write) {
        let mut count: usize = 0;
        loop {
            let millis = platform::micros();
            self.batch(serial);
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
                    "FPS: {}.{} (avg {}.{}) {}",
                    inst / 10,
                    inst % 10,
                    avg / 10,
                    avg % 10, CORE1_STATUS.load(Ordering::Relaxed)
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
