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
- `captures/phase2-session-20260326T161759Z/summary.txt`
- `captures/phase2-session-20260326T161759Z/baseline-compare.txt`

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
- the first structured Phase 2 session confirmed the refreshed baseline still
  matches `artifacts/local-probe/`

Phase 1 interpretation:

- The instrumentation tooling, output format, storage layout, and repeated-run
  workflow now exist in the repo.
- The next work should move into Phase 2 protocol mapping while treating
  runtime permissions and host `usbmon` availability as environment follow-up
  tasks rather than blockers for the instrumentation layer itself.

## 2026-03-26 Runtime and usbmon Evidence

Command provenance:

- `sudo ./scripts/install-device-access.sh install`
- `sudo ./scripts/enable-usbmon.sh apply`
- `SYNAPTICS_USBMON_USE_SUDO=1 ./scripts/run-phase2-session.sh`

Artifacts:

- `captures/phase2-session-20260326T173939Z/runtime-probe.txt`
- `captures/phase2-session-20260326T174318Z/summary.txt`
- `captures/phase2-session-20260326T174835Z/summary.txt`
- `captures/phase2-session-20260326T174835Z/runtime-probe.txt`
- `captures/phase2-session-20260326T174835Z/usbmon/metadata.txt`
- `captures/phase2-session-20260326T174835Z/usbmon/usbmon-bus1.txt`
- `captures/phase2-session-20260326T174835Z/usbmon-analysis.md`

Confirmed facts:

- The host can now open the target device and claim interface `0` without a
  kernel driver attached.
- A bounded runtime trace against endpoint `0x83` completed successfully at the
  transport level, but all observed reads timed out without payload.
- A target-scoped usbmon capture now exists for bus `1`, device `3`.
- The captured target traffic contains one standard control IN transfer on
  endpoint `0x00` with request bytes `80 00 0000 0000 0002` and response
  payload `00 00`; the analyzer now identifies this as a standard USB
  `GET_STATUS` request.
- The captured target traffic contains repeated interrupt IN submissions on
  endpoint `0x83` with requested length `64`, followed by completion records
  with status `-2` and actual length `0`.
- The repeated `0x83` poll loop is highly regular, with submit cadence around
  `250ms` and completion latency around `250ms` in the repeated captures.
- No bulk OUT traffic on `0x01` and no bulk IN traffic on `0x81` were observed
  in the current capture window.
- A longer 15-second capture with an explicit touch-hold-release sequence still
  produced only the standard `GET_STATUS` exchange plus empty `0x83` timeout
  completions, reinforcing that the device remains in a quiet pre-activation
  state under the current Linux-side interaction.
- An explicit bulk-IN trace on `0x81` produced repeated bulk timeout
  completions with no payload, while sysfs showed `power/runtime_status`
  changing from `suspended` before the run to `active` after it.
- A combined run with runtime PM forced on plus back-to-back `0x83` interrupt
  and `0x81` bulk traces still produced only timeout completions with zero
  payload on both endpoints, and no `0x01` OUT traffic.
- The forced runtime PM override restored cleanly after the session; the
  current host state is again `power/control: auto` and
  `power/runtime_status: suspended`.

Phase 2 interpretation:

- The repo now has real runtime and bus-capture evidence for the target reader,
  not just enumeration artifacts.
- The currently observed traffic is consistent with a quiet or stimulus-gated
  interrupt endpoint rather than a chatty idle status stream.
- The device can be nudged out of USB runtime suspend by explicit host access,
  but that still does not expose a vendor protocol exchange.
- For this Linux-side path, forcing runtime PM on is still not sufficient to
  transition the reader into an active vendor session.
- The protocol is still not mapped far enough to classify the device as
  image-based or match-on-chip because no vendor bulk exchange has been
  observed yet.
