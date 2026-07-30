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

/// Frames between live FPS reports.
///
/// The cost is not the serial port: at playable speed this is a few lines a
/// second against a bulk endpoint good for tens of KB/s. It is the two `{:.1}`
/// conversions, and `f64` is soft-float on the M33 (its FPU is
/// single-precision), so `flt2dec` is the expensive part. Even so this stays
/// well under the full-precision `format!` that `set_diagnostics` below already
/// pays on *every* frame, so lowering it further costs less than that one line
/// does.
const FPS_REPORT_INTERVAL: u32 = 15;

pub struct Emulator<I: Input, O: Output> {
    cpu: Cpu,
    bus: Bus,
    ppu: Ppu,
    output: O,
    input: I,
    /// Running FPS total. Kept as a sum rather than a Vec of per-frame samples:
    /// the Vec grew unbounded (~96 KB over a 12000-frame run) for no benefit.
    fps_total: f64,
    fps_frames: u32,
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
            fps_total: 0.0,
            fps_frames: 0,
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
                let _ = writeln!(diag, "Avg FPS: {}", self.fps_total / self.fps_frames as f64);
                break;
            }
            if !self.output.refresh() {
                break;
            }

            let diff = platform::micros() - millis;

            let time = 1_000_000.0 / diff as f64;
            self.fps_total += time;
            self.fps_frames += 1;
            if time < 1_000_000.0 / 60.0 {
                //sleep(Duration::from_micros((1_000_000.0 / 60.0 - time) as u64))
            }
            // Live readout. Emitted between frames, so it cannot interrupt a
            // partially written line of the ROM's own serial output.
            if self.fps_frames % FPS_REPORT_INTERVAL == 0 {
                let avg = self.fps_total / self.fps_frames as f64;
                let _ = writeln!(diag, "FPS: {time:.1} (avg {avg:.1})");
            }
            self.output.set_diagnostics(format!("FPS: {}", time));
        }
    }
}
