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

Exit criteria:

- the team can open the dev container and inspect the reader reliably
- documentation and setup no longer describe the old HID/I2C path

## Phase 1: Instrumentation

Purpose:
Create the tooling needed to inspect the device safely and reproducibly.

Deliverables:

- USB descriptor dump tooling
- endpoint inventory and device profile
- capture workflow for `usbmon`
- structured storage for captures and notes

Exit criteria:

- we can record and replay the same basic hardware facts across sessions

## Phase 2: Protocol Mapping

Purpose:
Understand how the reader behaves on the wire.

Deliverables:

- packet framing notes
- command and response hypotheses
- event/state transition notes
- distinction between image capture, template operations, and match-on-chip
  behavior

Exit criteria:

- we can explain the major packet classes and predict basic device behavior

## Phase 3: Userspace Prototype

Purpose:
Build a small driver-like prototype that can talk to the device directly.

Deliverables:

- userspace transport layer
- logging and trace-friendly command execution
- safe handshake flow
- basic device interaction beyond enumeration

Exit criteria:

- the prototype can complete at least one meaningful reader operation

## Phase 4: Enrollment Path

Purpose:
Prove that Linux can enroll and verify fingerprints with this device.

Deliverables:

- mapped enrollment flow
- mapped verification flow
- handling for errors, retries, and finger presence events
- documented limits of the device model

Exit criteria:

- we can demonstrate reliable enroll and verify on the target laptop

## Phase 5: `libfprint` Integration

Purpose:
Move from a local prototype to the Linux fingerprint stack.

Deliverables:

- a `libfprint` integration strategy
- prototype or patch set aligned with the correct `libfprint` device model
- testing notes for `fprintd`

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

Exit criteria:

- new devices are added because protocol evidence supports them, not because
  their vendor name matches
