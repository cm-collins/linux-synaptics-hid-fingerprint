# Upstream Path

This document keeps the long-term upstreaming strategy for the project.

Despite the filename, the current plan is not a kernel driver first. The
project is now centered on a vendor-specific USB fingerprint reader on the
HP EliteBook x360 1040 G7:

- Reader: Synaptics FS7604 Touch Fingerprint Sensor with PurePrint
- USB ID: `06CB:00E9`

## Current Upstream Strategy

The preferred path is:

1. understand the device protocol in userspace
2. build a local prototype
3. integrate with `libfprint`
4. validate with `fprintd`
5. upstream the support in the most appropriate Linux userspace project

Kernel work is a later option, not the default assumption.

## Why Kernel-First Is Not The Plan

Facts observed on the target machine:

- the reader enumerates over USB
- the interface class is vendor-specific
- there is no useful generic fingerprint kernel binding for the device today
- `fprintd` reports `No devices available`

That points toward `libfprint` and protocol work first.

## Decision Gates

### Gate 1: Transport Confidence

We should not write a driver until we can explain:

- interface layout
- endpoint roles
- basic device states
- startup behavior

### Gate 2: Device Model

We need to determine whether the reader behaves like:

- an image-based device
- a template-oriented device
- a match-on-chip device

That choice affects the right `libfprint` integration model.

### Gate 3: Upstream Target

After the device model is clear, choose the upstream path:

- `libfprint` if the device can be supported in the Linux fingerprint stack
- helper tooling only if the device is too constrained for full stack support
- kernel work only if userspace support is blocked by missing kernel behavior

## Long-Term Milestones

### Milestone 1: Grounding

- confirm stable enumeration facts for `06CB:00E9`
- align docs and development environment

### Milestone 2: Instrumentation

- add descriptor and capture tooling
- collect repeatable traces

### Milestone 3: Protocol Understanding

- document command framing and device state transitions

### Milestone 4: Prototype

- implement a small userspace transport and command harness

### Milestone 5: Linux Integration

- make enrollment and verification work through the Linux fingerprint stack

## Eventual Kernel Work

Kernel work is still possible later, but only if evidence shows it is required.
If that day comes, the likely areas are:

- permissions or enumeration helpers
- transport support that cannot be handled cleanly from userspace
- platform integration gaps discovered during `libfprint` work

## References

- [libfprint driver development documentation](https://fprint.freedesktop.org/libfprint-dev/driver-dev.html)
- [libfprint advanced topics](https://fprint.freedesktop.org/libfprint-stable/advanced-topics.html)
- [Linux USB subsystem documentation](https://www.kernel.org/doc/html/latest/driver-api/usb/index.html)
