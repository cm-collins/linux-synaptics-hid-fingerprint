# linux-synaptics-hid-fingerprint

> Research workspace for bringing Synaptics fingerprint support to Linux,
> starting with the HP EliteBook x360 1040 G7 fingerprint reader
> `USB VID:06CB PID:00E9`.

[![CI](https://github.com/cm-collins/linux-synaptics-hid-fingerprint/actions/workflows/ci.yml/badge.svg)](https://github.com/cm-collins/linux-synaptics-hid-fingerprint/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-GPL--2.0-blue)
![Status](https://img.shields.io/badge/status-phase%201%20complete-green)
![Target](https://img.shields.io/badge/target-libfprint%20userspace-green)

## Goal

Build upstream-quality Linux support for an unsupported Synaptics fingerprint
reader family by starting with one real device and one real laptop:

- Laptop: HP EliteBook x360 1040 G7
- Reader: Synaptics FS7604 Touch Fingerprint Sensor with PurePrint
- USB ID: `06CB:00E9`

The current path is:

1. Ground the project around this exact hardware.
2. Reverse engineer the device in userspace over USB.
3. Prove enrollment and verification can work on Linux.
4. Move toward `libfprint` and `fprintd` integration.
5. Only consider kernel work later if the protocol truly requires it.

## Current Facts

- The laptop exposes the reader as a vendor-specific USB device, not as a
  normal `hidraw` fingerprint device.
- `lsusb` sees `06cb:00e9`.
- The interface has one bulk OUT endpoint, one bulk IN endpoint, and one small
  interrupt IN endpoint.
- `fprintd-enroll` currently reports `No devices available`.

That makes this a userspace reverse-engineering and `libfprint` project first,
not a kernel HID/I2C project.

## Project Docs

- [Architecture](./docs/ARCHITECTURE.md)
- [Build Readiness](./docs/BUILD_READINESS.md)
- [Phases](./docs/PHASES.md)
- [Supported Devices](./docs/SUPPORTED_DEVICES.md)
- [Upstream Path](./docs/KERNEL_SUBMISSION.md)
- [Assumptions And Unknowns](./notes/ASSUMPTIONS.md)
- [Evidence Checklist](./notes/EVIDENCE_CHECKLIST.md)
- [Evidence Ledger](./notes/EVIDENCE_LEDGER.md)
- [Experiment Journal](./notes/EXPERIMENT_JOURNAL.md)
- [Protocol Mapping Notes](./notes/PROTOCOL_MAPPING.md)
- [Phase 2 Capture Checklist](./notes/PHASE2_CAPTURE_CHECKLIST.md)

## Current Focus

Phase 1 instrumentation is now in place. The current focus is moving into early
Phase 2 protocol mapping while preserving repeatable baseline evidence.

The repository now provides:

- collecting stable facts about `06CB:00E9`
- a repeatable local probe workflow
- a checked-in device profile and evidence ledger
- stable artifact paths for repeated baseline runs
- a `usbmon` capture workflow with host-side preflight checks

The most useful next outputs are:

- idle and stimulus-driven bus captures
- command and response hypotheses
- startup-state observations after successful interface claim
- evidence that narrows the correct `libfprint` device model

The repo now includes an initial CLI for that work:

```bash
cargo run -- probe
```

It can also save a stable report and perform bounded interface probing:

```bash
cargo run -- probe --output artifacts/probe.txt
cargo run -- probe --claim 0 --read-ep 0x83 --length 64 --timeout-ms 250
cargo run -- device-profile --output notes/device-profile.md
./scripts/compare-baseline-runs.sh artifacts/local-probe artifacts/local-probe-replay
./scripts/capture-usbmon.sh 5
./scripts/run-phase2-session.sh
```

## Development Workflow

### Prerequisites

- A Linux machine where the `06CB:00E9` reader is physically attached
- Rust toolchain
- `libusb-1.0` development files

Optional:

- VS Code with the Dev Containers extension
- Docker or Docker Engine

### Run On A Local Linux Machine

Install the Rust and USB prerequisites for your distro, then run:

```bash
./scripts/run-local-probe.sh
```

That helper captures a baseline `lsusb` view, saves a filtered `usb-devices`
snapshot when available, captures sysfs metadata, runs the Rust probe, and refreshes
`notes/device-profile.md`.

You can still run the commands manually if you prefer:

```bash
cargo run -- probe
cargo run -- device-profile --output notes/device-profile.md
./scripts/capture-usbmon.sh 5
./scripts/run-phase2-session.sh
```

If the reader is attached locally, `probe` should enumerate `06cb:00e9`
directly through `libusb`.

### Open In Dev Container

```bash
git clone https://github.com/cm-collins/linux-synaptics-hid-fingerprint
cd linux-synaptics-hid-fingerprint
code .
```

Then reopen the workspace in the dev container.

If you change the dev container Cargo paths or volume mounts, rebuild the
container so the writable cache and target directories are recreated for
`devuser`.

### What The Container Is For

The container is set up for:

- Rust-based USB tooling
- `libusb` and `libfprint` development
- USB inspection with `lsusb`, `usb-devices`, and sysfs
- protocol capture preparation with `usbmon`
- analysis and documentation work

The CLI also works inside the container, but successful live USB probing still
depends on the host exposing the fingerprint reader to the container.
`usbmon` capture also depends on the host exposing debugfs and the `usbmon`
interfaces.

### First Commands To Run

```bash
lsusb -d 06cb:00e9
usb-devices | sed -n '/Vendor=06cb ProdID=00e9/,+20p'
fprintd-enroll -f right-index-finger
cargo run -- probe
cargo run -- device-profile
./scripts/capture-usbmon.sh 5
./scripts/run-phase2-session.sh
```

Expected result on a local Linux host with the reader attached:

- the USB device is visible
- `fprintd` still says `No devices available`

Expected result in a dev container without USB passthrough:

- the tool explains that `libusb` cannot currently see `06cb:00e9`
- the diagnostic output lists the USB devices the container can see

That is the Phase 1 baseline the repository now preserves.

## Repository Direction

This repository is intentionally device-first.

We are not claiming broad support for all Synaptics readers yet. The immediate
goal is to make one unsupported reader family understandable and testable, then
grow support outward from there.

## Contributing

Useful contributions right now:

- USB descriptor analysis
- packet capture review
- `libfprint` driver research
- documentation cleanup
- testing on closely related Synaptics readers

When sharing captures, avoid publishing real fingerprint data unless we have a
clear redaction and privacy workflow.

## License

GPL-2.0
