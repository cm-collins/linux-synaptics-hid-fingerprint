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

## 2026-03-26 Phase 2 Workflow Setup

Goal:

- create a repeatable session workflow for protocol-mapping attempts

Command:

- `./scripts/run-phase2-session.sh`

Result:

- the repository now includes a structured Phase 2 session script
- each session can refresh baseline evidence, compare it with the stored
  baseline, and record runtime-probe and usbmon preflight results in one place
- follow-up: run the script on the host after enabling the necessary USB
  permissions and, when possible, `usbmon`

## 2026-03-26 First Phase 2 Session

Goal:

- validate the new Phase 2 session workflow against the real host

Command:

- `./scripts/run-phase2-session.sh`

Result:

- session artifacts were created under
  `captures/phase2-session-20260326T161759Z/`
- the refreshed baseline compared cleanly against `artifacts/local-probe/`
- the bounded runtime probe still failed with
  `Access denied (insufficient permissions)`
- the usbmon preflight still failed because
  `/sys/kernel/debug/usb/usbmon/1u` is not visible

## 2026-03-26 Host Access Enablement

Goal:

- enable safe host access for runtime probing and usbmon capture

Command:

- `sudo ./scripts/install-device-access.sh install`
- `sudo ./scripts/enable-usbmon.sh apply`

Result:

- the host now exposes `/sys/kernel/debug/usb/usbmon/1u`
- the host now permits non-destructive runtime access to the target device
- follow-up: rerun the Phase 2 session workflow with concurrent runtime and
  usbmon capture

## 2026-03-26 Successful Runtime Claim

Goal:

- observe the device immediately after interface claim using bounded reads on
  interrupt endpoint `0x83`

Command:

- `SYNAPTICS_USBMON_USE_SUDO=1 ./scripts/run-phase2-session.sh`

Result:

- session artifacts were created under
  `captures/phase2-session-20260326T173939Z/`
- interface `0` claim succeeded with no kernel driver attached
- repeated interrupt reads on `0x83` timed out cleanly with no payload
- follow-up: capture the same activity on usbmon and introduce a user touch
  stimulus during the trace window

## 2026-03-26 First Successful usbmon Capture

Goal:

- confirm that concurrent bus capture works for the target reader

Command:

- `SYNAPTICS_USBMON_USE_SUDO=1 ./scripts/run-phase2-session.sh`

Result:

- session artifacts were created under
  `captures/phase2-session-20260326T174318Z/`
- runtime probing and usbmon both completed successfully
- the first usbmon analysis did not yet isolate meaningful target-device
  traffic, which led to a parser hardening pass and target-device filtering
- follow-up: rerun with an explicit user touch attempt during the capture

## 2026-03-26 Touch-Stimulus Capture

Goal:

- observe whether a user touch changes bus traffic or interrupt behavior on the
  target reader

Command:

- `SYNAPTICS_USBMON_USE_SUDO=1 ./scripts/run-phase2-session.sh`
- user touch stimulus during the capture window

Result:

- session artifacts were created under
  `captures/phase2-session-20260326T174835Z/`
- target-device traffic for bus `1`, device `3` was captured successfully
- one standard control IN transfer on endpoint `0x00` returned `00 00`
- repeated interrupt IN polls on `0x83` completed with timeout status `-2` and
  zero payload
- no bulk traffic on `0x01` or `0x81` was observed in this capture window
- interpretation: the simple touch attempt did not trigger a visible vendor
  protocol exchange or interrupt payload in the captured window

## 2026-03-26 Repeated Touch-Stimulus Capture

Goal:

- confirm whether the first touch-stimulus result was a fluke and measure the
  target reader's observed poll timing more precisely

Command:

- `SYNAPTICS_USBMON_USE_SUDO=1 ./scripts/run-phase2-session.sh`

Result:

- session artifacts were created under
  `captures/phase2-session-20260326T175215Z/`
- the analyzer confirmed the endpoint `0x00` control transfer is a standard USB
  `GET_STATUS` request
- the analyzer confirmed a steady `0x83` interrupt poll cadence of roughly
  `250ms`, with matching completion latency and zero payload
- no bulk traffic on `0x01` or `0x81` was observed again
- interpretation: the current Linux-side interaction still looks like a
  pre-activation or uninitialized state rather than a vendor protocol session

## 2026-03-26 Extended Touch-Hold-Release Capture

Goal:

- extend the capture window and confirm whether a longer touch-hold-release
  sequence triggers the first vendor exchange

Command:

- `SYNAPTICS_USBMON_USE_SUDO=1 SYNAPTICS_USBMON_DURATION=15 SYNAPTICS_RUNTIME_ITERATIONS=24 SYNAPTICS_SESSION_NOTE="touch, hold, and release during one capture window" ./scripts/run-phase2-session.sh`

Result:

- session artifacts were created under
  `captures/phase2-session-20260326T180415Z/`
- the analyzer again found only a standard USB `GET_STATUS` control request on
  endpoint `0x00`
- endpoint `0x83` again showed repeated interrupt polls with timeout completions
  and zero payload across the longer capture window
- no bulk traffic on `0x01` or `0x81` was observed
- interpretation: even the longer manual touch-hold-release interaction did not
  transition the device into an active vendor protocol state

## 2026-03-26 Explicit Bulk-IN Trace

Goal:

- check whether direct reads against bulk endpoint `0x81` reveal payloads that
  are not visible in the interrupt-focused trace

Command:

- `SYNAPTICS_USBMON_USE_SUDO=1 SYNAPTICS_RUNTIME_ENDPOINT=0x81 SYNAPTICS_RUNTIME_TRANSPORT=bulk SYNAPTICS_RUNTIME_LENGTH=64 SYNAPTICS_RUNTIME_TIMEOUT_MS=250 SYNAPTICS_RUNTIME_ITERATIONS=24 SYNAPTICS_RUNTIME_DELAY_MS=500 SYNAPTICS_SESSION_NOTE="bulk IN trace on endpoint 0x81 during touch-hold-release" ./scripts/run-phase2-session.sh`

Result:

- session artifacts were created under
  `captures/phase2-session-20260326T181336Z/`
- usbmon captured repeated bulk-IN submissions on endpoint `0x81`, each
  completing with timeout status `-2` and zero payload
- the analyzer still found only a standard USB `GET_STATUS` control request on
  endpoint `0x00`
- sysfs snapshots showed `power/runtime_status: suspended` before the run and
  `power/runtime_status: active` after the run
- interpretation: explicit bulk access wakes the device at the USB runtime
  power layer, but still does not trigger the first visible vendor bulk
  response

## 2026-03-26 Forced Runtime-PM Dual Trace

Goal:

- test whether forcing USB runtime PM on and tracing both `0x83` and `0x81`
  within one capture window is enough to unlock the first vendor exchange

Command:

- `SYNAPTICS_USBMON_USE_SUDO=1 SYNAPTICS_FORCE_RUNTIME_PM_ON=1 SYNAPTICS_RUNTIME_PM_USE_SUDO=1 SYNAPTICS_RUNTIME_ENDPOINT=0x83 SYNAPTICS_RUNTIME_TRANSPORT=interrupt SYNAPTICS_RUNTIME_ITERATIONS=24 SYNAPTICS_RUNTIME_DELAY_MS=500 SYNAPTICS_SECONDARY_ENDPOINT=0x81 SYNAPTICS_SECONDARY_TRANSPORT=bulk SYNAPTICS_SECONDARY_ITERATIONS=24 SYNAPTICS_SECONDARY_DELAY_MS=500 SYNAPTICS_SESSION_NOTE="force runtime PM on; trace 0x83 then 0x81 during touch-hold-release" ./scripts/run-phase2-session.sh`

Result:

- session artifacts were created under
  `captures/phase2-session-20260326T182401Z/`
- the primary interrupt trace on `0x83` showed only timeout completions with
  zero payload
- the secondary bulk trace on `0x81` also showed only timeout completions with
  zero payload
- usbmon showed no `0x01` OUT traffic and no vendor control setup packets
- the runtime-PM force-on override restored cleanly after the run
- interpretation: even with runtime PM forced on and both known IN endpoints
  traced back to back, the device still does not enter a visible vendor
  protocol session on this Linux-side path
