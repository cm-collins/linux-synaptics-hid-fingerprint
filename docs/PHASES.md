# Phases

## Phase 0: Grounding

Purpose:
Establish a clean factual baseline for the target reader and shape the project
around the real transport and integration path.

Deliverables:

- target device confirmed as `USB VID:06CB PID:00E9`
- repo docs aligned around the HP EliteBook x360 1040 G7 focus device
- development container prepared for USB and `libfprint` work
- repeatable commands for device enumeration and baseline checks
- a clear decision record that the project is userspace-first
- tracked assumptions, hypotheses, and unknowns
- a placeholder evidence ledger and artifact layout

Exit criteria:

- the team can open the dev container and inspect the reader reliably
- documentation and setup no longer describe the old HID/I2C path
- the repo distinguishes confirmed facts from inferred claims

## Phase 1: Instrumentation

Purpose:
Create the tooling needed to inspect the device safely and reproducibly.

Status note:

- As of 2026-03-26, the repository contains the Phase 1 instrumentation
  deliverables and the supporting evidence workflow.
- See `notes/EVIDENCE_LEDGER.md`, `notes/EXPERIMENT_JOURNAL.md`,
  `scripts/run-local-probe.sh`, `scripts/capture-usbmon.sh`, and
  `scripts/compare-baseline-runs.sh`.

Deliverables:

- USB descriptor dump tooling
- endpoint inventory and device profile
- capture workflow for `usbmon`
- structured storage for captures and notes
- first Rust instrumentation crate or workspace
- stable output format for repeated baseline runs

Exit criteria:

- we can record and replay the same basic hardware facts across sessions
- the same inspection workflow yields comparable artifacts on repeated runs

## Phase 2: Protocol Mapping

Purpose:
Understand how the reader behaves on the wire.

Deliverables:

- packet framing notes
- command and response hypotheses
- event/state transition notes
- distinction between image capture, template operations, and match-on-chip
  behavior
- a running experiment journal with command provenance
- explicit labels for "confirmed", "likely", and "unknown"

Exit criteria:

- we can explain the major packet classes and predict basic device behavior
- we can point to stored captures that support each major protocol claim

## Phase 3: Userspace Prototype

Purpose:
Build a small driver-like prototype that can talk to the device directly.

Deliverables:

- userspace transport layer
- logging and trace-friendly command execution
- safe handshake flow
- basic device interaction beyond enumeration
- guardrails for read timeouts, retries, and bounded probing

Exit criteria:

- the prototype can complete at least one meaningful reader operation
- the prototype can recover cleanly from expected probe failures

## Phase 4: Enrollment Path

Purpose:
Prove that Linux can enroll and verify fingerprints with this device.

Deliverables:

- mapped enrollment flow
- mapped verification flow
- handling for errors, retries, and finger presence events
- documented limits of the device model
- evidence showing whether biometric operations happen on-host or on-device

Exit criteria:

- we can demonstrate reliable enroll and verify on the target laptop

## Phase 5: `libfprint` Integration

Purpose:
Move from a local prototype to the Linux fingerprint stack.

Deliverables:

- a `libfprint` integration strategy
- prototype or patch set aligned with the correct `libfprint` device model
- testing notes for `fprintd`
- documented rationale for the chosen `libfprint` model

Exit criteria:

- `fprintd` can see and use the device through the new support path

## Phase 6: Broaden Support

Purpose:
Carefully extend support to adjacent Synaptics reader families.

Deliverables:

- compatibility matrix
- device family clustering by protocol similarity
- evidence-based support for additional IDs such as `00B7`, `00F0`, `00BD`,
  and `00FC`
- comparison notes showing what is shared versus device-specific

Exit criteria:

- new devices are added because protocol evidence supports them, not because
  their vendor name matches
