# Captures

This directory stores sanitized traffic captures and metadata for the
`06CB:00E9` target.

## Layout

- `usbmon-<timestamp>/metadata.txt`
- `usbmon-<timestamp>/usbmon-bus<bus>.txt`

The raw text capture is bus-scoped. Use the metadata file to correlate the
capture with the target bus number, device number, and sysfs path observed
during the same run.

## Workflow

1. Make sure the host can see the fingerprint reader.
2. Enable `usbmon` on the host if needed:
   `sudo modprobe usbmon`
3. Make sure debugfs is mounted on the host:
   `sudo mount -t debugfs none /sys/kernel/debug`
4. Run `./scripts/capture-usbmon.sh 5`
5. Review and sanitize the resulting text capture before sharing or committing
   it.

## Privacy

Avoid committing captures that contain real biometric payloads unless the data
has been reviewed and redacted appropriately.
