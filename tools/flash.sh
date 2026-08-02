#!/usr/bin/env bash
#
# Build, flash and attach to rusty-gb on a Pico, without touching the board.
# The macOS counterpart of tools/flash.ps1 -- same loop, same flags:
#
#     find the port -> touch it at 1200 baud -> wait for BOOTSEL ->
#     cargo build -> picotool load -x -> wait for the port to come back ->
#     stream it to this terminal
#
# The BOOTSEL button is the only part of the flashing loop that needs a human,
# and it is the part that makes an unplug-replug cycle out of every build. The
# firmware watches the CDC port for the 1200 baud touch (see src/usb_serial.rs),
# so this script can do that reset itself.
#
# Ctrl-C in the monitor exits; the board keeps running.
#
# Usage:
#   tools/flash.sh                      build, reset, flash, attach
#   tools/flash.sh --pico1              target a Pico 1 / W / WH (RP2040)
#   tools/flash.sh --pico2              target a Pico 2 (RP2350) -- the default
#   tools/flash.sh --monitor-only       attach to a board that is already running
#   tools/flash.sh --no-build           flash what is already in target/
#   tools/flash.sh --no-monitor         flash and exit
#   tools/flash.sh --reset              just drop into BOOTSEL, then exit
#   tools/flash.sh --port /dev/cu.usbmodem101
#
set -euo pipefail

# Which board to build for. Only the build and the load care -- everything
# from the 1200 baud touch to the monitor is identical on both, because it is
# our own firmware answering rather than anything board-specific.
#
# There is no autodetection, and cannot usefully be: a board running rusty-gb
# advertises the same VID/PID whichever chip it is, so the two are
# indistinguishable until one is already in BOOTSEL -- by which point the build
# has to have happened. Flashing an RP2040 image onto a Pico 2 is not
# dangerous; picotool refuses it on the family ID in the image.
BOARD='pico2'
CARGO_PROFILE='embedded'
REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

# usb_serial.rs advertises Raspberry Pi's VID with the SDK's CDC PID. ioreg
# prints both in decimal: 0x2e8a, 0x000a.
USB_VID=11914
USB_PID=10

# Both ends of the reboot: BOOTSEL takes about a second to enumerate, and the
# emulator re-appearing after `picotool load -x` is a second reboot on top.
BOOTSEL_WAIT_SECONDS=15
PORT_WAIT_SECONDS=20

PORT=''
NO_BUILD=0
NO_MONITOR=0
MONITOR_ONLY=0
RESET_ONLY=0

step() { printf '\033[36m==> %s\033[0m\n' "$1"; }
note() { printf '\033[33m==> %s\033[0m\n' "$1"; }
die()  { printf '\033[31merror: %s\033[0m\n' "$1" >&2; exit 1; }

usage() {
    # The header above, down to the first line that is not a comment, so this
    # cannot drift out of step with it.
    awk 'NR > 2 { if (!/^#/) exit; sub(/^# ?/, ""); print }' "${BASH_SOURCE[0]}"
    exit "${1:-0}"
}

while [ $# -gt 0 ]; do
    case "$1" in
        -p|--port)      [ $# -ge 2 ] || die "--port needs a device path"; PORT="$2"; shift 2 ;;
        --pico1)        BOARD='pico1'; shift ;;
        --pico2)        BOARD='pico2'; shift ;;
        --no-build)     NO_BUILD=1; shift ;;
        --no-monitor)   NO_MONITOR=1; shift ;;
        --monitor-only) MONITOR_ONLY=1; shift ;;
        --reset)        RESET_ONLY=1; shift ;;
        -h|--help)      usage 0 ;;
        *)              printf 'unknown option: %s\n\n' "$1" >&2; usage 1 ;;
    esac
done

# Resolved after parsing rather than in the case above, so that the last of
# --pico1/--pico2 wins instead of whichever one happened to set TARGET last.
case "$BOARD" in
    pico1) TARGET='thumbv6m-none-eabi';        BOARD_NAME='Pico 1 / W / WH (RP2040)'; BOOTSEL_VOLUME='RPI-RP2'  ;;
    pico2) TARGET='thumbv8m.main-none-eabihf'; BOARD_NAME='Pico 2 (RP2350)';          BOOTSEL_VOLUME='RP2350'   ;;
esac

ELF="$REPO_ROOT/target/$TARGET/$CARGO_PROFILE/rusty-gb"

[ "$(uname -s)" = 'Darwin' ] || die "This script is macOS-only: it finds the board with ioreg and drives the port with BSD stty. On Linux the port is /dev/ttyACM* (or /dev/serial/by-id/) and stty takes -F, not -f."

# Walk the USB tree for a device with our VID/PID and take the callout device of
# the CDC interface underneath it. ioreg prints each device and its descendants
# depth-first -- IOUSBHostDevice -> interface -> ACM -> IOSerialBSDClient -- so
# the VID/PID in scope when an IOCalloutDevice appears are the ones that own it,
# as long as scope is closed again on the way back out. That is what the indent
# tracking is for: through a hub, the board's subtree sits inside another
# IOUSBHostDevice's, and without it the hub's next serial child would inherit
# the board's identity.
#
# /dev/cu.* is the callout node -- unlike /dev/tty.*, opening it does not wait
# for carrier, which nothing on a CDC port ever raises.
find_board_port() {
    local found
    found=$(ioreg -r -c IOUSBHostDevice -l -w 0 2>/dev/null | awk -v vid="$USB_VID" -v pid="$USB_PID" '
        /\+-o / {
            indent = index($0, "+-o")
            if (dev_indent > 0 && indent <= dev_indent) { dev_vid = ""; dev_pid = ""; dev_indent = 0 }
            if (/<class IOUSBHostDevice/) { dev_vid = ""; dev_pid = ""; dev_indent = indent }
        }
        /"idVendor" =/        { dev_vid = $NF }
        /"idProduct" =/       { dev_pid = $NF }
        /"IOCalloutDevice" =/ {
            if (dev_vid == vid && dev_pid == pid) {
                gsub(/"/, "", $NF)
                print $NF
                exit
            }
        }
    ') || true

    if [ -n "$found" ]; then
        printf '%s\n' "$found"
        return 0
    fi

    # Fallback for a machine where the ioreg walk comes up empty: one CDC port,
    # unambiguously the board. Two or more and we refuse to guess -- pass --port.
    # Counted rather than collected into an array, which bash 3.2 -- still what
    # /bin/bash is on macOS -- treats as unset when empty under `set -u`.
    local candidate count=0 only=''
    for candidate in /dev/cu.usbmodem*; do
        if [ -e "$candidate" ]; then
            count=$(( count + 1 ))
            only="$candidate"
        fi
    done
    if [ "$count" -eq 1 ]; then
        note "No 2E8A:000A in ioreg; using the only CDC port, $only" >&2
        printf '%s\n' "$only"
        return 0
    fi

    return 1
}

wait_for_board_port() {
    local deadline=$(( SECONDS + $1 )) found
    while [ "$SECONDS" -lt "$deadline" ]; do
        if found=$(find_board_port); then
            # Enumerating and being ready to open are not the same instant.
            sleep 0.3
            printf '%s\n' "$found"
            return 0
        fi
        sleep 0.25
    done
    return 1
}

# `picotool info` exits non-zero when there is no device in BOOTSEL, which is
# the ordinary case here rather than a failure. Exit code is the only signal we
# want from it.
bootsel_present() {
    picotool info >/dev/null 2>&1
}

# Opening at 1200 baud and closing again is the whole protocol: the close drops
# DTR, and the firmware sees the pair -- 1200 baud line coding with DTR low --
# in its USB interrupt and calls the ROM's reboot-to-BOOTSEL. stty opens the
# port, sets the speed and closes, which is exactly that sequence.
bootsel_touch() {
    local port="$1"
    # Checked separately, because a --port that does not exist is a typo and
    # should say so rather than spend the BOOTSEL timeout finding out.
    [ -e "$port" ] || die "No such port: $port"
    # Past that, the board is already rebooting by the time stty closes the
    # handle, so a non-zero exit here is as often success as failure. The
    # BOOTSEL poll that follows is what actually decides.
    stty -f "$port" 1200 >/dev/null 2>&1 || true
}

enter_bootsel() {
    # Checked here rather than at the top so that --monitor-only, which needs
    # nothing but a serial port, works on a machine without picotool.
    command -v picotool >/dev/null 2>&1 ||
        die "picotool is not on PATH. brew install picotool"

    if bootsel_present; then
        step 'Board is already in BOOTSEL.'
        return 0
    fi

    local port="$PORT"
    if [ -z "$port" ]; then
        port=$(find_board_port) ||
            die "No rusty-gb board found, in BOOTSEL or on a CDC port. Plug it in -- or hold BOOTSEL while plugging it in if its firmware predates the 1200 baud reset."
    fi

    step "Rebooting $port into BOOTSEL (1200 baud touch)"
    bootsel_touch "$port"

    local deadline=$(( SECONDS + BOOTSEL_WAIT_SECONDS ))
    while [ "$SECONDS" -lt "$deadline" ]; do
        bootsel_present && return 0
        sleep 0.3
    done

    die "Board did not appear in BOOTSEL within ${BOOTSEL_WAIT_SECONDS}s. Hold BOOTSEL while replugging, then re-run with --no-build if the build is current."
}

do_build() {
    step "$BOARD_NAME: cargo build --target $TARGET --profile $CARGO_PROFILE"
    ( cd "$REPO_ROOT" && cargo build --target "$TARGET" --profile "$CARGO_PROFILE" ) ||
        die 'cargo build failed'
}

do_load() {
    [ -f "$ELF" ] || die "No ELF at $ELF -- build it first (drop --no-build)."
    step "picotool load $(basename "$ELF")"
    picotool load -u -v -x -t elf "$ELF" || die 'picotool load failed'
}

start_monitor() {
    local port="$PORT"
    if [ -z "$port" ]; then
        port=$(wait_for_board_port "$PORT_WAIT_SECONDS") ||
            die "Board never came back on a CDC port within ${PORT_WAIT_SECONDS}s."
    fi

    # Hold the port open on fd 3 for the whole session. That is what keeps DTR
    # asserted -- the firmware waits up to 10 s for a host to raise it before
    # starting, so this is also what releases the boot banner -- and it keeps
    # the termios below from being reset, which the driver does on last close.
    # Braces, not a subshell: fd 3 has to land in this shell, but the open's own
    # complaint should not reach the terminal ahead of the message below.
    { exec 3<>"$port"; } 2>/dev/null || die "Could not open $port."
    trap 'exec 3>&- 2>/dev/null || true; printf "\n"' EXIT

    # Baud is ignored by CDC; the rest is not. Without `raw` the line discipline
    # rewrites the firmware's CRLFs and echoes, which mangles the FPS line.
    stty -f "$port" 115200 raw -echo clocal >/dev/null 2>&1 || true

    step "Attached to $port -- Ctrl-C to detach"

    # The reader runs in the background and this shell waits on it, rather than
    # running it in the foreground: bash defers a trap until the foreground
    # child returns, so an INT trap around a foreground `cat` only fires because
    # a terminal Ctrl-C signals the whole process group and kills the `cat` too.
    # Waiting is interruptible, so this exits on the signal however it arrives.
    cat <&3 &
    local reader=$!
    # Ctrl-C is how you leave the monitor, not a failure: take the reader with
    # us and skip the "went away" line, since the board is still running.
    trap 'kill "$reader" 2>/dev/null || true; exit 0' INT
    wait "$reader" || true

    note "$port went away."
}

if [ "$MONITOR_ONLY" -eq 1 ]; then
    start_monitor
    exit 0
fi

if [ "$RESET_ONLY" -eq 1 ]; then
    enter_bootsel
    step "In BOOTSEL. Drag a .uf2 onto the $BOOTSEL_VOLUME volume, or run picotool."
    exit 0
fi

[ "$NO_BUILD" -eq 1 ] || do_build
enter_bootsel
do_load
[ "$NO_MONITOR" -eq 1 ] || start_monitor
