# rusty-gb

A Game Boy (DMG) emulator in Rust, targeting both a desktop host and a
Raspberry Pi Pico 2 (RP2350, Cortex-M33) from one source tree.

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

What *is* included is `test-roms/gb-test-roms-master/`, blargg's freely
redistributable test suite. That is all the test suite needs.

## Building

The host is the default target, so the test suite is always one command away:

```sh
cargo test              # 34 tests, incl. blargg cpu_instrs 1-11
cargo run --release     # needs test-roms/Pokemon Red.gb
```

The Pico 2 is one `--target` away:

```sh
cargo build --target thumbv8m.main-none-eabihf --profile embedded
cargo run   --target thumbv8m.main-none-eabihf --profile embedded  # flashes via picotool
```

`cargo run` for the Pico uses `picotool`, which needs the board in BOOTSEL
mode — hold BOOTSEL while plugging it in.

The split is on `target_os = "none"`: the Pico target reports `none`, hosts
report macos/linux/windows, so neither build needs a remembered `--features`
to be correct.

## Talking to the Pico

Diagnostics go out over **USB CDC**, not defmt/RTT, so reading them needs no
debug probe — just the cable that already powers the board:

```sh
ls /dev/tty.usbmodem*            # macOS; /dev/ttyACM* on Linux
screen /dev/tty.usbmodem101      # baud is ignored for CDC; exit with Ctrl-A K
```

You get a live FPS line every 15 frames plus a final average. The emulator
waits up to 10 seconds for a terminal to open the port before starting, so
nothing is lost off the top, and still boots unattended if nothing attaches.

The Game Boy's own serial port goes to the same physical port but a separate
sink, so a parser reading cartridge output never sees an FPS line spliced into
it.
