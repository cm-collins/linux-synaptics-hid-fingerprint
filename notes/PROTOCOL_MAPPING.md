# Protocol Mapping

This note is the working home for Phase 2 observations about wire behavior,
state transitions, and protocol hypotheses for the `06CB:00E9` target.

## Current Baseline

Confirmed transport facts:

- USB ID: `06CB:00E9`
- Device class/subclass/protocol: `ff/10/ff`
- Interface `0`, alternate setting `0`
- Endpoints:
  `0x01` OUT bulk, `0x81` IN bulk, `0x83` IN interrupt
- Driver binding: `none`

Primary evidence:

- `notes/device-profile.md`
- `notes/EVIDENCE_LEDGER.md`
- `artifacts/local-probe/probe.txt`
- `artifacts/local-probe/sysfs-device.txt`
- `artifacts/local-probe/sysfs-interface.txt`

## Packet Classes

Use this section to record packet families once captures exist.

| Class | Direction | Endpoint | Length Pattern | Confidence | Evidence |
|---|---|---|---|---|---|
| Standard control status read | IN | `0x00` | fixed `2` bytes | confirmed | `captures/phase2-session-20260326T175215Z/usbmon-analysis.md` |
| Idle interrupt poll | IN | `0x83` | submit `64`, complete `0` on timeout; remains empty even with runtime PM forced on | confirmed | `captures/phase2-session-20260326T182401Z/usbmon-analysis.md` |
| Host command request | OUT | `0x01` | unknown | unknown | no `0x01` traffic observed yet |
| Device bulk response poll | IN | `0x81` | submit `64`, complete `0` on timeout at ~750ms cadence; remains empty even with runtime PM forced on | confirmed | `captures/phase2-session-20260326T182401Z/usbmon-analysis.md` |

## State Transitions

Record observed state changes here.

| Trigger | Expected State Change | Observed Result | Confidence | Evidence |
|---|---|---|---|---|
| Device enumerates | descriptor-visible idle state | confirmed | confirmed | baseline artifacts |
| Interface claim | unknown | claim succeeds; device enters a stable interrupt-poll loop on `0x83` with ~250ms completion latency and no payload | confirmed | `captures/phase2-session-20260326T180415Z/runtime-probe.txt`, `captures/phase2-session-20260326T180415Z/usbmon-analysis.md` |
| Finger touch | unknown | no visible change in `0x83` traffic during short or longer touch-hold-release capture attempts so far | likely | `captures/phase2-session-20260326T180415Z/usbmon-analysis.md` |
| Explicit bulk-IN trace on `0x81` | unknown | device transitions from `runtime_status: suspended` to `active`, but `0x81` still returns no payload | confirmed | `captures/phase2-session-20260326T181336Z/sysfs-before/sysfs-device.txt`, `captures/phase2-session-20260326T181336Z/sysfs-after/sysfs-device.txt`, `captures/phase2-session-20260326T181336Z/usbmon-analysis.md` |
| Force runtime PM on; trace `0x83` then `0x81` | unknown | both endpoints remain silent with timeout completions only; forced-on state is later restored to `auto` | confirmed | `captures/phase2-session-20260326T182401Z/usbmon-analysis.md`, `captures/phase2-session-20260326T182401Z/runtime-pm/after-restore.txt` |

## Hypotheses

- The small interrupt endpoint `0x83` appears to be an event or status path
  driven by a steady host poll loop of roughly 250ms, but it stays silent at
  idle and under the simple touch attempts captured so far.
- Bulk OUT `0x01` and bulk IN `0x81` are likely the main command/response
  transport pair.
- The only observed control exchange so far is a standard USB `GET_STATUS` read
  on endpoint `0x00`, not a vendor protocol handshake.
- The longer 15-second touch-hold-release capture still did not reveal any
  `0x01` bulk OUT, `0x81` bulk IN, or non-empty `0x83` interrupt payload.
- An explicit bulk-IN trace on `0x81` can wake the device from USB runtime
  suspend to active state, but still does not produce any bulk payload without
  an additional activation step.
- Even after forcing `power/control=on` and tracing `0x83` then `0x81` back to
  back, no payloads or host-to-device vendor writes were observed.
- The current device model could still be image-based, event-based, or
  match-on-chip; no bulk payload evidence distinguishes them yet.

## Immediate Questions

- Does `0x83` emit payloads only after a stronger, longer, or software-driven
  finger stimulus?
- What user-space action causes the first `0x01` bulk command or `0x81` bulk
  response?
- Are there deterministic startup packets after claim or after the first touch?
- Does Windows send a minimal handshake before the sensor becomes active?

## Capture Sources

Add each capture session here once you have one.

| Date | Session Path | Scenario | Notes |
|---|---|---|---|
| 2026-03-26 | `captures/phase2-session-20260326T161759Z/` | idle preflight | baseline matched stored artifacts; runtime probe blocked by USB permissions; usbmon unavailable on host |
| 2026-03-26 | `captures/phase2-session-20260326T173939Z/` | first successful runtime claim | interface `0` claim succeeded; repeated `0x83` reads timed out with no payload |
| 2026-03-26 | `captures/phase2-session-20260326T174318Z/` | first successful usbmon run | usbmon capture succeeded but did not isolate meaningful target-device traffic |
| 2026-03-26 | `captures/phase2-session-20260326T174835Z/` | usbmon with user touch stimulus | captured target device `1:003`; observed standard control read on endpoint `0x00` and repeated `0x83` interrupt polls completing with timeout and zero payload |
| 2026-03-26 | `captures/phase2-session-20260326T175215Z/` | repeated touch-stimulus capture | confirms standard `GET_STATUS` on endpoint `0x00`; confirms steady `0x83` host poll cadence of roughly 250ms with no payload or bulk traffic |
| 2026-03-26 | `captures/phase2-session-20260326T180415Z/` | 15-second touch-hold-release capture | confirms the same quiet state over a longer window: standard `GET_STATUS` only, repeated `0x83` timeout completions, no vendor bulk traffic |
| 2026-03-26 | `captures/phase2-session-20260326T181336Z/` | explicit bulk-IN trace on `0x81` | confirms repeated `0x81` bulk-IN timeout completions with no payload; sysfs shows `runtime_status` changing from `suspended` to `active` during the run |
| 2026-03-26 | `captures/phase2-session-20260326T182401Z/` | force runtime PM on and trace `0x83` then `0x81` | confirms both endpoints stay silent even with `power/control=on`; runtime PM restore returns the device to `auto` afterward |

## Next Commands

```bash
SYNAPTICS_USBMON_USE_SUDO=1 SYNAPTICS_FORCE_RUNTIME_PM_ON=1 SYNAPTICS_RUNTIME_PM_USE_SUDO=1 SYNAPTICS_RUNTIME_ENDPOINT=0x83 SYNAPTICS_RUNTIME_TRANSPORT=interrupt SYNAPTICS_SECONDARY_ENDPOINT=0x81 SYNAPTICS_SECONDARY_TRANSPORT=bulk SYNAPTICS_SECONDARY_ITERATIONS=24 SYNAPTICS_SESSION_NOTE="force runtime PM on; trace 0x83 then 0x81 during touch-hold-release" ./scripts/run-phase2-session.sh
cargo run -- analyze-usbmon --input captures/phase2-session-20260326T182401Z/usbmon/usbmon-bus1.txt --bus 1 --device 3 --output captures/phase2-session-20260326T182401Z/usbmon-analysis.md
```
