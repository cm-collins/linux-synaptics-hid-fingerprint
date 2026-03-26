# Evidence Checklist

This checklist defines the minimum evidence the project should gather before
deeper protocol or driver claims are treated as grounded.

Current status and dated references live in:

- `notes/EVIDENCE_LEDGER.md`
- `notes/EXPERIMENT_JOURNAL.md`

## Device Identity

- `lsusb -d 06cb:00e9` output saved
- `usb-devices` block for the target saved
- sysfs path and interface metadata recorded
- kernel binding state recorded

## USB Topology

- device descriptor captured
- configuration descriptor captured
- interface number confirmed
- endpoint addresses, directions, and transfer types recorded
- max packet sizes recorded

## Runtime Behavior

- idle-state capture recorded
- behavior after interface claim recorded
- behavior after repeated reopen recorded
- cold boot versus warm boot differences noted
- interrupt endpoint behavior characterized at idle

## Probe Safety

- initial probing commands are bounded by timeouts
- reads and writes are logged with timestamps
- probe notes identify which actions are expected to be safe
- any risky or stateful commands are isolated and documented before reuse

## Product Direction

- evidence supports userspace-first transport access
- the likely `libfprint` device model is discussed explicitly
- unknowns that block enrollment support are listed plainly

## Exit Signal

This checklist is in good shape when the repo contains enough artifacts and
notes that another engineer can reproduce the baseline without guessing what
was observed.
