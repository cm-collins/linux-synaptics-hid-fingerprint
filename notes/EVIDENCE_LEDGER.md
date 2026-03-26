# Evidence Ledger

This ledger records the concrete evidence gathered for the `06CB:00E9` target
and points to the artifacts that support each claim.

## 2026-03-26 Local Baseline

Command provenance:

- `./scripts/run-local-probe.sh`
- `cargo run -- probe --claim 0 --read-ep 0x83 --length 64 --timeout-ms 250`
- host checks against `/sys/bus/usb/devices/1-7` and `/sys/bus/usb/devices/1-7:1.0`

Artifacts:

- `artifacts/local-probe/lsusb.txt`
- `artifacts/local-probe/usb-devices.txt`
- `artifacts/local-probe/probe.txt`
- `artifacts/local-probe/sysfs-device.txt`
- `artifacts/local-probe/sysfs-interface.txt`
- `notes/device-profile.md`

Confirmed facts:

- The reader enumerates as `06cb:00e9` on bus `001`, device `003`.
- The device presents one configuration with one interface.
- Interface `0`, alternate setting `0`, exposes three endpoints:
  `0x01` OUT bulk, `0x81` IN bulk, and `0x83` IN interrupt.
- The device reports USB `2.00`, EP0 max packet size `8`, remote wakeup
  enabled, and max power `100mA`.
- The interface driver binding is currently `none`.
- The sysfs device path is `/sys/bus/usb/devices/1-7`.

Environment notes:

- The baseline descriptor probe works from the host and produces stable text
  artifacts.
- A bounded runtime probe currently fails without elevated USB permissions with
  `Access denied (insufficient permissions)`.
- `usbmon` is not currently visible under `/sys/kernel/debug/usb/usbmon`, so
  the repo includes the workflow and preflight checks but no checked-in bus
  trace yet.

Phase 1 interpretation:

- The instrumentation tooling, output format, storage layout, and repeated-run
  workflow now exist in the repo.
- The next work should move into Phase 2 protocol mapping while treating
  runtime permissions and host `usbmon` availability as environment follow-up
  tasks rather than blockers for the instrumentation layer itself.
