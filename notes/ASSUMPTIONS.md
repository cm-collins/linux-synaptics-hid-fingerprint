# Assumptions And Unknowns

This file keeps the project honest about what is confirmed, what is inferred,
and what is still unknown.

Update it whenever a hypothesis becomes supported or gets disproven.

## Confirmed

- The current focus device is Synaptics `06CB:00E9`.
- The target laptop is the HP EliteBook x360 1040 G7.
- The reader enumerates as a vendor-specific USB device.
- `fprintd` does not currently support the device in the baseline environment.
- The immediate project direction is userspace-first rather than kernel-first.

## Strong Working Assumptions

- A safe first prototype can be built in userspace using direct USB access.
- The correct long-term integration target is likely `libfprint`.
- Narrow device-first work will produce better results than family-wide support
claims early on.
- Repeatable captures will matter more than speculative reverse engineering.

## Unknowns

- the exact startup handshake
- whether the device is image-based, event-based, or match-on-chip
- whether initialization depends on host-side secrets or signed exchanges
- whether idle interrupt traffic conveys finger presence or status events
- whether Windows performs setup that Linux must emulate
- whether full upstream `libfprint` support is feasible for this reader family

## Rules For Updating This File

- Move items to `Confirmed` only when an artifact or direct observation supports
the claim.
- Keep uncertain claims in `Strong Working Assumptions` or `Unknowns`.
- When possible, add dates and artifact references in the commit that updates
this file.

