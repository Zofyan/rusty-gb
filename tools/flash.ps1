#Requires -Version 5.1
<#
.SYNOPSIS
    Build, flash and attach to rusty-gb on a Pico 2, without touching the board.

.DESCRIPTION
    The BOOTSEL button is the only part of the Pico 2 flashing loop that needs a
    human, and it is the part that makes an unplug-replug cycle out of every
    build. The firmware watches the USB CDC port for the 1200 baud touch (see
    src/usb_serial.rs), so this script can do that reset itself:

        find the COM port -> touch it at 1200 baud -> wait for BOOTSEL ->
        cargo build -> picotool load -x -> wait for the port to come back ->
        stream it to this console

    Ctrl-C in the monitor exits; the board keeps running.

.PARAMETER Port
    COM port of the board, e.g. COM5. Found automatically by USB VID/PID when
    omitted, which is what you want unless two boards are plugged in.

.PARAMETER NoBuild
    Flash whatever is already in target/, skipping cargo.

.PARAMETER NoMonitor
    Flash and exit, instead of attaching to the port afterwards.

.PARAMETER MonitorOnly
    Attach to a board that is already running. Builds and flashes nothing.

.PARAMETER Reset
    Reboot the board into BOOTSEL and exit -- for when you want to drag a .uf2
    onto it by hand, or hand it to another tool.

.EXAMPLE
    .\tools\flash.ps1
    .\tools\flash.ps1 -MonitorOnly
    .\tools\flash.ps1 -NoBuild -NoMonitor
#>
[CmdletBinding()]
param(
    [string]$Port,
    [switch]$NoBuild,
    [switch]$NoMonitor,
    [switch]$MonitorOnly,
    [switch]$Reset
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# PowerShell 7 does not load System.IO.Ports by default; Windows PowerShell 5.1
# has it in System.dll already and fails this call harmlessly.
# Add-Type -AssemblyName System.IO.Ports -ErrorAction SilentlyContinue

$Target       = 'thumbv8m.main-none-eabihf'
$CargoProfile = 'embedded'
$RepoRoot    = Split-Path -Parent $PSScriptRoot
$Elf         = Join-Path $RepoRoot "target\$Target\$CargoProfile\rusty-gb"

# usb_serial.rs advertises Raspberry Pi's VID with the SDK's CDC PID, so this is
# what the board looks like to Windows while the emulator is running.
$UsbId       = 'USB\VID_2E8A&PID_000A'

# Both ends of the reboot: BOOTSEL takes about a second to enumerate, and the
# emulator re-appearing after `picotool load -x` is a second reboot on top.
$BootselWaitSeconds = 15
$PortWaitSeconds    = 20

function Write-Step([string]$Message) {
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Find-BoardPort {
    $device = Get-CimInstance -ClassName Win32_PnPEntity -ErrorAction SilentlyContinue |
        Where-Object { $_.PNPDeviceID -like "$UsbId*" -and $_.Name -match '\(COM\d+\)' } |
        Select-Object -First 1

    if ($device -and $device.Name -match '\((COM\d+)\)') { return $Matches[1] }
    return $null
}

function Wait-ForBoardPort([int]$TimeoutSeconds) {
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while ((Get-Date) -lt $deadline) {
        $found = Find-BoardPort
        if ($found) {
            # Enumerating and being ready to open are not the same instant.
            Start-Sleep -Milliseconds 300
            return $found
        }
        Start-Sleep -Milliseconds 250
    }
    return $null
}

function Test-BootselPresent {
    # `picotool info` exits non-zero when there is no device in BOOTSEL, which
    # is the ordinary case here rather than a failure -- and "no device" goes to
    # stderr, which under $ErrorActionPreference = 'Stop' would otherwise be
    # promoted to a terminating NativeCommandError. Exit code is the only signal
    # we want from it.
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & picotool info *> $null
    } finally {
        $ErrorActionPreference = $previous
    }
    return ($LASTEXITCODE -eq 0)
}

function Invoke-BootselTouch([string]$ComPort) {
    # Opening at 1200 baud with DTR low is the whole protocol: the firmware sees
    # both in its USB interrupt and calls the ROM's reboot-to-BOOTSEL.
    $serial = New-Object System.IO.Ports.SerialPort($ComPort, 1200, 'None', 8, 'One')
    $serial.DtrEnable = $false
    $serial.RtsEnable = $false
    try {
        $serial.Open()
        Start-Sleep -Milliseconds 150
    } catch {
        throw "Could not open $ComPort for the 1200 baud touch: $($_.Exception.Message)"
    } finally {
        # The board is already rebooting by now, so the close frequently throws
        # on a handle whose device has gone. That is success, not a failure.
        try { $serial.Close() } catch { }
        $serial.Dispose()
    }
}

function Enter-Bootsel {
    # Checked here rather than at the top of the script so that -MonitorOnly,
    # which needs nothing but a serial port, works on a machine without it.
    if (-not (Get-Command picotool -ErrorAction SilentlyContinue)) {
        throw 'picotool is not on PATH. Install it from https://github.com/raspberrypi/pico-sdk-tools/releases and put it somewhere on PATH.'
    }

    if (Test-BootselPresent) {
        Write-Step 'Board is already in BOOTSEL.'
        return
    }

    $comPort = if ($Port) { $Port } else { Find-BoardPort }
    if (-not $comPort) {
        throw "No rusty-gb board found, in BOOTSEL or on a COM port. Plug it in -- or hold BOOTSEL while plugging it in if its firmware predates the 1200 baud reset."
    }

    Write-Step "Rebooting $comPort into BOOTSEL (1200 baud touch)"
    Invoke-BootselTouch $comPort

    $deadline = (Get-Date).AddSeconds($BootselWaitSeconds)
    while ((Get-Date) -lt $deadline) {
        if (Test-BootselPresent) { return }
        Start-Sleep -Milliseconds 300
    }

    throw "Board did not appear in BOOTSEL within $BootselWaitSeconds s. Hold BOOTSEL while replugging, then re-run with -NoBuild if the build is current."
}

function Invoke-Build {
    Write-Step "cargo build --target $Target --profile $CargoProfile"
    Push-Location $RepoRoot
    try {
        & cargo build --target $Target --profile $CargoProfile
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
    } finally {
        Pop-Location
    }
}

function Invoke-Load {
    if (-not (Test-Path $Elf)) {
        throw "No ELF at $Elf -- build it first (drop -NoBuild)."
    }

    Write-Step "picotool load $(Split-Path -Leaf $Elf)"
    & picotool load -u -v -x -t elf $Elf
    if ($LASTEXITCODE -ne 0) { throw "picotool load failed ($LASTEXITCODE)" }
}

function Start-Monitor {
    $comPort = if ($Port) { $Port } else { Wait-ForBoardPort $PortWaitSeconds }
    if (-not $comPort) {
        throw "Board never came back on a COM port within $PortWaitSeconds s."
    }

    # Baud is ignored by CDC. DTR is not: the firmware waits up to 10 s for a
    # host to assert it before starting, so that the first frames are not
    # emitted into a port nobody has opened.
    $serial = New-Object System.IO.Ports.SerialPort($comPort, 115200, 'None', 8, 'One')
    $serial.DtrEnable = $true
    $serial.ReadTimeout = 200

    Write-Step "Attached to $comPort -- Ctrl-C to detach"
    try {
        $serial.Open()
        while ($serial.IsOpen) {
            try {
                $chunk = $serial.ReadExisting()
            } catch {
                # Unplugged, or rebooted into BOOTSEL from another window.
                Write-Host "`n==> $comPort went away." -ForegroundColor Yellow
                break
            }
            if ($chunk) { [Console]::Write($chunk) } else { Start-Sleep -Milliseconds 20 }
        }
    } finally {
        try { $serial.Close() } catch { }
        $serial.Dispose()
        Write-Host ''
    }
}

if ($MonitorOnly) {
    Start-Monitor
    return
}

if ($Reset) {
    Enter-Bootsel
    Write-Step 'In BOOTSEL. Drag a .uf2 onto the RP2350 drive, or run picotool.'
    return
}

if (-not $NoBuild) { Invoke-Build }
Enter-Bootsel
Invoke-Load
if (-not $NoMonitor) { Start-Monitor }
