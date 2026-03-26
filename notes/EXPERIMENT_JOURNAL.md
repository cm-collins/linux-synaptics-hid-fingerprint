# Experiment Journal

This journal records non-destructive experiments, command provenance, and the
result of each run.

## 2026-03-26 Baseline Enumeration

Goal:

- confirm stable descriptor and endpoint facts for `06CB:00E9`

Command:

- `./scripts/run-local-probe.sh`

Result:

- success
- baseline artifacts refreshed under `artifacts/local-probe/`
- descriptor output confirms one interface and three endpoints
- checked-in Markdown profile refreshed at `notes/device-profile.md`

## 2026-03-26 Runtime Probe Attempt

Goal:

- record behavior after interface claim using a bounded interrupt read

Command:

- `cargo run -- probe --claim 0 --read-ep 0x83 --length 64 --timeout-ms 250`

Result:

- host-side access to the device exists for enumeration
- opening the device for runtime probing failed with
  `Access denied (insufficient permissions)`
- follow-up: rerun with an appropriate host USB permission model or temporary
  elevated access before treating runtime behavior as characterized

## 2026-03-26 usbmon Preflight

Goal:

- verify whether the host currently exposes `usbmon` for bus-level capture

Command:

- `scripts/capture-usbmon.sh 5`

Result:

- preflight currently fails because `/sys/kernel/debug/usb/usbmon/<bus>u` is
  not visible on this machine
- follow-up: enable `usbmon` on the host with `sudo modprobe usbmon` and make
  sure debugfs is mounted before collecting the first bus trace
