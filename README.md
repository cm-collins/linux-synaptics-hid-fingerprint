# linux-synaptics-hid-fingerprint

> Research workspace for bringing Synaptics fingerprint support to Linux,
> starting with the HP EliteBook x360 1040 G7 fingerprint reader
> `USB VID:06CB PID:00E9`.

[![CI](https://github.com/cm-collins/linux-synaptics-hid-fingerprint/actions/workflows/ci.yml/badge.svg)](https://github.com/cm-collins/linux-synaptics-hid-fingerprint/actions/workflows/ci.yml)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-GPL--2.0-blue)
![Status](https://img.shields.io/badge/status-grounding%20%2F%20research-yellow)
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

## Current Focus

We are in the grounding phase. That means:

- collecting stable facts about `06CB:00E9`
- setting up a repeatable development container
- capturing descriptors, interface details, and baseline USB behavior
- documenting assumptions before writing protocol-specific code

The most useful outputs right now are:

- confirmed descriptor and endpoint facts with timestamps
- a list of hypotheses that still need evidence
- repeatable capture commands tied to saved artifacts
- a first Rust instrumentation tool for safe USB inspection

## Development Workflow

### Prerequisites

- VS Code with the Dev Containers extension
- Docker or Docker Engine
- A machine where the `06CB:00E9` reader is physically attached

### Open In Dev Container

```bash
git clone https://github.com/cm-collins/linux-synaptics-hid-fingerprint
cd linux-synaptics-hid-fingerprint
code .
```

Then reopen the workspace in the dev container.

### What The Container Is For

The container is set up for:

- Rust-based USB tooling
- `libusb` and `libfprint` development
- USB inspection with `lsusb`, `usb-devices`, and sysfs
- protocol capture preparation with `usbmon`
- analysis and documentation work

### First Commands To Run

```bash
lsusb -d 06cb:00e9
usb-devices | sed -n '/Vendor=06cb ProdID=00e9/,+20p'
fprintd-enroll -f right-index-finger
```

Expected result today:

- the USB device is visible
- `fprintd` still says `No devices available`

That is the baseline we want to improve.

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
