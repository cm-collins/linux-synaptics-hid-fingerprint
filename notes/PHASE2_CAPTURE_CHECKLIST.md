# Phase 2 Capture Checklist

Use this checklist before and during protocol-mapping experiments.

## Preflight

- confirm the reader is still visible with `lsusb -d 06cb:00e9`
- refresh baseline artifacts with `./scripts/run-local-probe.sh`
- confirm the sysfs path for the reader is stable
- record whether the interface driver is still `none`

## Runtime Access

- confirm whether direct open/claim requires elevated permissions
- if elevated access is required, document exactly how it was obtained
- keep runtime reads bounded by endpoint, length, and timeout

## usbmon

- enable `usbmon` on the host with `sudo modprobe usbmon`
- ensure debugfs is mounted with `sudo mount -t debugfs none /sys/kernel/debug`
- save each capture session under `captures/phase2-session-<timestamp>/`
- record the bus number and device number used during the capture

## Scenarios To Capture

- idle device with no touch interaction
- first interface claim attempt
- repeated reopen without touching the sensor
- finger touch or enrollment stimulus, if safe to test
- cold boot versus warm boot differences

## Notes To Record

- whether `0x83` shows idle interrupt traffic
- whether any bulk traffic appears before a host write
- whether response lengths look fixed, framed, or status-like
- whether the device state changes after a timeout or failed open

## Session Wrap-Up

- update `notes/EXPERIMENT_JOURNAL.md`
- update `notes/PROTOCOL_MAPPING.md`
- link the session path in `notes/EVIDENCE_LEDGER.md` if the evidence is strong
