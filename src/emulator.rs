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
    #[cfg_attr(target_os = "none", link_section = ".data.ram_func")]
    pub fn run(&mut self, max_cycles: usize, stdout: &mut dyn Write) {
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
                    write!(stdout, "{}", self.bus.registers.sb as char).expect("Couldn't write");
                    self.bus.registers.sc = 0;
                }
            }
            count += 1;
            if count > max_cycles && max_cycles != 0 {
                let avg = self.fps_total / self.fps_frames as f64;
                #[cfg(target_os = "none")]
                defmt::println!("Avg FPS: {=f64}", avg);
                #[cfg(not(target_os = "none"))]
                println!("Avg FPS: {}", avg);
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
            self.output.set_diagnostics(format!("FPS: {}", time));
        }
    }
}
