# rusty-gb

A Game Boy (DMG) emulator in Rust, targeting a desktop host and two Raspberry
Pi Picos from one source tree:

| target | triple | notes |
| --- | --- | --- |
| host | native | macOS/Linux/Windows; the default, so `cargo test` is one word |
| Pico 2 | `thumbv8m.main-none-eabihf` | RP2350, Cortex-M33, 520 KiB SRAM |
| Pico 1 / W / WH | `thumbv6m-none-eabi` | RP2040, Cortex-M0+, 264 KiB SRAM |

## Cartridge images are not included

No commercial ROMs are distributed with this repository. `test-roms/*.gb` is
gitignored — supply your own dumps of cartridges you own.

The host binary reads its ROM at runtime, but **the Pico build needs one at
compile time**: `src/main.rs` does

```rust
static ROM_IMAGE: &[u8] = include_bytes!("../test-roms/Pokemon Red.gb");
```

so that the image lands in `.rodata` and can be executed in place from XIP
flash, costing no SRAM. Drop a file at that path or edit the `include_bytes!`
to point at your own. Without it the Pico build fails at compile time with a
"couldn't read" error naming the missing path.

Note that a built Pico image therefore *embeds* the cartridge — don't
redistribute the resulting `.elf` or `.uf2`.

The cartridge has to fit in flash alongside the code, which is the one place
the Pico 1 is tighter than the Pico 2: 2 MiB against 4 MiB. The emulator itself
is about 64 KiB, so a 1 MiB cartridge like Pokémon Red leaves roughly 950 KiB
spare on a Pico 1 — but a 2 MiB cartridge will not fit there at all, and the
failure is a linker error about `FLASH` overflowing rather than anything
subtle.

What *is* included are the two freely redistributable test suites, vendored so
that `cargo test` needs no network and no setup:

* `test-roms/gb-test-roms-master/` — blargg's suite
* `test-roms/mooneye-test-suite/` — Gekkio's mooneye suite (MIT, `LICENSE`
  included), release `mts-20240926-1737-443f6e1`

## Building

The host is the default target, so the test suite is always one command away:

```sh
cargo test --release    # 69 tests; see Testing below for the other 125
cargo run --release     # needs test-roms/Pokemon Red.gb
```

`--release` is worth typing: the ROM suites run a few hundred million emulated
instructions, which takes about two seconds optimised and rather longer not.

Either Pico is one `--target` away:

```sh
# Pico 2
cargo build --target thumbv8m.main-none-eabihf --profile embedded
cargo run   --target thumbv8m.main-none-eabihf --profile embedded  # flashes via picotool

# Pico 1 / W / WH
cargo build --target thumbv6m-none-eabi --profile embedded
cargo run   --target thumbv6m-none-eabi --profile embedded
```

`cargo run` for either Pico uses `picotool`, which needs the board in BOOTSEL
mode. Either hold BOOTSEL while plugging it in, or — once firmware with the
1200 baud reset is on the board — let the host ask for it (see below).

`--target` is the whole of the board selection. Host versus Pico splits on
`target_os = "none"`, which the Pico triples report and macos/linux/windows do
not; Pico 1 versus Pico 2 splits on the `rp2040` / `rp2350` cfgs that
`build.rs` derives from the triple, which is also how it picks the linker
script out of `memory/`. So no build needs a remembered `--features` to be
correct, and there is no board default to get wrong.

Almost all of the difference is confined to `src/clocks/`, since the two HALs
are close enough in shape that everything else names `rusty_gb::hal` and stops
caring. What is left is a handful of `#[cfg]`s in `src/main.rs` and
`src/usb_serial.rs`, each on a peripheral the two PACs name differently.

### Both boards run overclocked

`src/clocks/` replaces each HAL's `init_clocks_and_plls`, which brings its part
up at the nominal speed, with the same sequence at double that. The emulator is
CPU-bound on both boards, so the frame rate follows the clock.

| | Pico 2 | Pico 1 / W / WH |
| --- | --- | --- |
| core | Cortex-M33 | Cortex-M0+ |
| stock | 150 MHz | 125 MHz |
| `SYS_MHZ` | 300 | 250 |
| core voltage | 1.15 V | 1.15 V |
| flash divisor | scaled with the clock | left alone |

`SYS_MHZ` in the file for your board is the only knob — set it to the stock
figure and the PLL config and core voltage follow, putting the board back where
the datasheet wants it.

The flash divisor is the one place the two genuinely differ. On the Pico 2 it
has to be scaled by hand to keep QSPI SCK where the bootrom left it, so flash
bandwidth stands still and the speedup falls short of a clean 2× by however
much the 16 KiB XIP cache misses. On the Pico 1 nothing needs correcting —
`boot2` leaves SCK at `clk_sys / 4`, which at 250 MHz is 62.5 MHz, well inside
what the board's flash is rated for — so flash speeds up along with the core.
Reconfiguring it there would mean a RAM-resident function anyway, because the
RP2040 un-maps XIP while its SSI is disabled. The comments in
`src/clocks/rp2040.rs` go into why.

Don't expect the Pico 1 to match the Pico 2 even at a similar clock. The M0+ is
16-bit Thumb only, with no DSP, no FPU and a multiply that is not always
single-cycle, so it does substantially less per cycle than the M33.

**Both defaults are out of spec.** Raspberry Pi specify 150 MHz for the RP2350
and 133 MHz for the RP2040, both at 1.10 V. Silicon varies, and a part that
will not hold this shows it as a hang or a corrupted frame, not as an error
message. If that happens, try 1.20 V — `VSEL` `0b01101` on the Pico 2,
`VSEL_A::VOLTAGE1_20` on the Pico 1 — before suspecting anything else, and drop
`SYS_MHZ` if it persists.

The first line out of the USB port names the board and the frequency the PLL
actually locked to, so a fallback is visible immediately rather than only as
disappointing FPS:

```
rusty-gb on Pico 1 (RP2040): clk_sys 250 MHz
```

## Testing

The emulator core is a library (`src/lib.rs`); `src/main.rs` is the two entry
points and the Pico's board glue. That split is what lets `tests/` and
`benches/` link the same code the binary runs.

```sh
cargo test --release                       # 69 tests, all passing
cargo test --release -- --ignored          # the 125 known-failing ROMs
cargo test --release --test mooneye        # one suite
```

There are four layers, in rough order of how much they will tell you:

| what | where | what it catches |
| --- | --- | --- |
| **ROM suites** | `tests/blargg.rs`, `tests/mooneye.rs` | the emulator disagreeing with real hardware |
| **snapshots** | `tests/snapshot.rs` | the emulator disagreeing with *itself* — any drift in cycles, pixels or registers |
| **opcode timing** | `src/cpu.rs`, `mod timing` | an instruction costing the wrong number of M-cycles |
| **unit tests** | throughout `src/` | the pieces, including a full opcode-dispatch equivalence check |

### The ROM suites, and why most of them are ignored

157 ROMs, one `#[test]` each so a regression names the ROM that broke. 32 pass
(15 blargg, 17 mooneye). The other 125 are `#[ignore]`d, each carrying the reason
it cannot pass rather than a bare "known failure":

```
#[ignore = "no APU (fail)"]
#[ignore = "PPU is stepped per instruction, not per dot (timeout)"]
#[ignore = "MBC5 not implemented (crashed)"]
```

Those reasons come down to two things. There is no APU at all, and the emulator
advances the PPU, the timer and the bus one *instruction* at a time rather than
one cycle at a time — so anything measuring when within an instruction an effect
lands will fail. Notably this is **not** a question of instruction cycle counts:
`mod timing` checks all 512 opcodes against the DMG's published costs, both arms
of every conditional branch included, and they are all correct.

To re-derive the list after a fix — which is the only way it stays honest:

```sh
cargo run --release --example test-report            # a table of every ROM
cargo run --release --example test-report -- mooneye # one suite
cargo run --release --example test-report -- --rust  # the declarations, to paste
```

The trade in using `#[ignore]` is that a ROM which *starts* passing stays quietly
ignored. `cargo test -- --ignored` is what surfaces those, and CI runs it on
every push as a non-blocking step for exactly that reason.

Two suites, two protocols, both in `src/testrom.rs`. blargg's `cpu_instrs`
generation reports over the serial port; everything later reports through
cartridge RAM at `$A000` and says nothing over serial at all. mooneye reports in
the register file at a `LD B,B` breakpoint. `every_vendored_rom_is_declared`
fails if a ROM is added to `test-roms/` without a test.

## Benchmarking

```sh
cargo bench                          # compare against the saved baseline
cargo bench -- --save main           # record this build as the baseline
cargo bench -- --json                # machine-readable
cargo bench -- cpu_instrs            # filter
```

Each workload runs a fixed number of *emulated* cycles, so every repetition does
identical work — the benchmark asserts as much — and the fastest of seven is the
estimator. Three ROMs against two output backends separate interpreter cost from
the cost of handing scanlines to a backend.

```
workload                 Mcycles/s    x real       fps    Minstr/s  spread
--------------------------------------------------------------------------
cpu_instrs/dummy              82.4      78.6      4696        39.2    2.6%
cpu_instrs/frame              79.7      76.0      4539        37.8    2.5%
oam_bug/dummy                 93.2      88.8      5306        38.8    8.1%
```

**`x real` is the number that matters**: emulated M-cycles per second over the
1048576 a DMG manages. It is the only figure that compares across hosts, across
ROMs, and against the Pico.

Baselines live in `target/bench-baselines/`, so they are per-machine and
per-build and are not checked in. A difference under 3% is reported as
unchanged; wall-clock timing on a general-purpose host does not resolve better
than that.

### The FPS the binary prints is not frames per second

`Emulator::run` reports one "frame" per call to `Emulator::batch`, and a batch is
17476 *instructions*, not a frame's worth of cycles. The benchmark measures the
actual ratio — about 2.25 M-cycles per instruction, so a batch is around 2.25
frames of emulated time. The live readout over USB CDC is therefore roughly a
third of the frames really being emulated. The number is consistent with itself
and fine for spotting a slowdown; it is just not fps.

## Talking to the Pico

Diagnostics go out over **USB CDC**, not defmt/RTT, so reading them needs no
debug probe — just the cable that already powers the board:

```sh
ls /dev/cu.usbmodem*             # macOS; /dev/ttyACM* on Linux
screen /dev/cu.usbmodem101       # baud is ignored for CDC; exit with Ctrl-A K
tools/flash.sh --monitor-only    # or this, which finds the port itself
```

`/dev/cu.*` rather than `/dev/tty.*`: the callout node does not wait on carrier
detect, which nothing on a CDC port ever raises.

You get a live FPS line every 15 frames plus a final average. The emulator
waits up to 10 seconds for a terminal to open the port before starting, so
nothing is lost off the top, and still boots unattended if nothing attaches.

The first line out of the port names the board and the system clock the PLL
actually locked to, so an overclock that silently fell back is visible
immediately rather than only as disappointing FPS. Both builds enumerate under
the same VID/PID, so this line is also the only thing that says which image is
on the board in front of you.

The Game Boy's own serial port goes to the same physical port but a separate
sink, so a parser reading cartridge output never sees an FPS line spliced into
it.

## Flashing without the BOOTSEL button

The firmware watches the CDC port for the 1200 baud touch — open the port at
1200 baud with DTR low and it reboots into BOOTSEL — so the host can put the
board into flashing mode itself. `picotool` then loads over the same cable, and
the terminal comes back on its own.

Two scripts drive the whole loop, one per host. Same steps, same flags:

```sh
tools/flash.sh                  # macOS: build, reset, flash, attach — Pico 2
tools/flash.sh --pico1          # the same, for a Pico 1 / W / WH
tools/flash.sh --monitor-only   # attach to a board that is already running
tools/flash.sh --no-build       # flash what is already in target/
tools/flash.sh --reset          # just drop into BOOTSEL, then exit
```

```powershell
.\tools\flash.ps1              # Windows, same five
.\tools\flash.ps1 -Pico1
.\tools\flash.ps1 -MonitorOnly
.\tools\flash.ps1 -NoBuild
.\tools\flash.ps1 -Reset
```

Both find the board by USB VID/PID (`2E8A:000A`), so no port has to be
remembered — `ioreg` on macOS, PnP device IDs on Windows. Pass `--port
/dev/cu.usbmodem101` (`-Port COM5`) if two boards are plugged in. Both need
`picotool` on `PATH` — `brew install picotool` on macOS — except under
`--monitor-only`, which needs nothing but the port. Ctrl-C detaches the monitor
and leaves the board running.

`--pico1`/`-Pico1` only changes what gets built and loaded; the reset, the
port discovery and the monitor are identical either way, because it is our own
firmware answering rather than anything board-specific. There is no
autodetection and cannot usefully be — a board running rusty-gb looks the same
whichever chip it is, and by the time one is in BOOTSEL the build has already
had to happen. Getting it wrong is not dangerous: `picotool` refuses an image
whose family ID does not match the board.

The monitor holds the port open for the whole session rather than reopening it
per read, because dropping DTR is half of the reset signal and the firmware's
wait-for-host watches the other half.

The same trick by hand, which is all the script is doing:

```sh
stty -f /dev/cu.usbmodem101 1200       # macOS; stty -F /dev/ttyACM0 1200 on Linux
cargo run --target thumbv8m.main-none-eabihf --profile embedded   # or thumbv6m-none-eabi
```

On Linux that one-liner is the whole story — there is no `tools/flash.sh`
equivalent there, since port discovery is `/dev/serial/by-id/` rather than
`ioreg` and `stty` takes `-F`.

A board whose firmware predates this still needs the button held at plug-in —
including the first flash after checking this out.
