# Architecture

## Summary

The project is centered on one unsupported device first:

- Laptop: HP EliteBook x360 1040 G7
- Reader: Synaptics FS7604 Touch Fingerprint Sensor with PurePrint
- USB ID: `06CB:00E9`

The working assumption is that support should begin in userspace over USB and
move toward `libfprint` integration, not from a kernel HID/I2C driver.

## Why This Architecture

Observed behavior on the target laptop:

- `lsusb` sees the reader as `06cb:00e9`
- the device exposes a vendor-specific USB interface
- no kernel driver is bound to the interface
- `fprintd` cannot use the device yet

Because of that, the safest architecture is:

1. USB transport inspection and capture
2. protocol mapping
3. userspace prototype
4. `libfprint` integration
5. optional upstreaming beyond the local prototype

## Main Flow

The main engineering flow for this repo is:

1. Enumerate the USB device and store reproducible facts.
2. Inspect descriptors, interfaces, and endpoint layout.
3. Capture idle traffic and stimulus-driven traffic.
4. Identify command framing, status messages, and device state transitions.
5. Build a userspace prototype that can safely talk to the reader.
6. Decide whether the device is:
   image-based, event-based, or match-on-chip.
7. Integrate the result into the `libfprint` model that fits the device.
8. Validate enrollment and verification through `fprintd`.

## Layered View

### Layer 1: Host And Device

- HP EliteBook x360 1040 G7
- Synaptics `06CB:00E9`
- Linux host kernel
- USB bus and sysfs metadata

### Layer 2: Research Tooling

- `lsusb`
- `usb-devices`
- sysfs under `/sys/bus/usb/devices`
- `usbmon` for bus-level captures
- custom Rust tooling built on `libusb`

### Layer 3: Protocol Understanding

- descriptor decoding
- endpoint semantics
- packet framing
- request and response mapping
- state machine notes

### Layer 4: Driver Prototype

- Rust or C prototype that opens the USB interface directly
- safe command execution and logging
- device-specific experiments

### Layer 5: Linux Integration

- `libfprint` backend or driver integration
- `fprintd` enrollment and verification
- distro-friendly usage path

## Repository Layout

The repo should evolve toward this shape:

- `docs/`
  project plans, architecture notes, device matrix, protocol findings
- `scripts/`
  environment setup and capture helpers
- `captures/`
  sanitized USB logs and descriptor dumps
- `notes/`
  protocol notes and experiment journals
- `src/`
  userspace tooling and prototype code

Not every directory exists yet. The order matters less than keeping the flow
device-first and evidence-driven.

## Design Principles

- Device-first, not family-first
- Userspace-first, not kernel-first
- Evidence before implementation
- Repeatable captures over one-off experiments
- Privacy-aware handling of biometric data
- Upstream alignment where practical

## Non-Goals For Now

- claiming support for all Synaptics readers
- building a generic fingerprint framework before the protocol is understood
- forcing a kernel driver path without proof it is necessary
- publishing raw biometric data casually
