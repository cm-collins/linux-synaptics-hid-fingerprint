# Build Readiness

This document answers a practical question:

Are the repository docs strong enough to start building?

The answer is yes.

The repo now has enough instrumentation, artifact layout, and evidence notes to
treat Phase 1 as complete and move into early Phase 2 protocol mapping.

## Current Readiness

The repository is ready for:

- development container setup
- USB device enumeration
- baseline descriptor collection
- repeatable local baseline capture
- usbmon capture preflight and workflow
- comparison of repeated baseline runs
- continued protocol experiment planning

The repository is not yet ready for:

- confident protocol implementation
- direct `libfprint` driver work
- broad support claims across Synaptics readers
- assumptions that the Windows behavior has already been explained

## What Is Strong Already

- one concrete hardware target: `06CB:00E9`
- a coherent userspace-first architecture
- a sensible phase model from grounding to integration
- a development environment aimed at USB and `libfprint` work
- a checked-in device profile and evidence ledger
- repeatable baseline artifacts and comparison tooling
- explicit non-goals that keep scope controlled

## What Still Blocks Deeper Driver Work

The remaining gaps are now mostly protocol and environment gaps, not missing
instrumentation scaffolding:

- bus-level traces captured with `usbmon`
- a host permission model that allows safe runtime interface claims
- protocol notes explaining startup behavior and response framing
- a justified `libfprint` device model choice

## Recommended Next Build Target

The first implementation target has been met with a small Rust tool focused on
safe inspection and stable report generation.

The next implementation target should focus on protocol mapping support:

- collect and review idle traffic
- capture stimulus-driven traffic safely
- document startup behavior after successful interface claim
- begin classifying command and response families

## Decision Gates Before Deeper Driver Work

Before protocol-specific work accelerates, the project should have evidence for:

- exact interface numbers and endpoint addresses
- startup behavior after interface claim
- whether interrupt traffic appears while idle
- whether device responses are deterministic across cold and warm boots
- whether any command framing can be identified without destructive probing

## Definition Of "Grounded Enough To Build"

The project is grounded enough to move beyond instrumentation because all of
the following are now true:

- the target device profile is stored in the repo
- capture commands produce reproducible artifacts
- assumptions are tracked explicitly
- the first probing tool can reproduce the same baseline facts on demand
- the team can explain what is known versus guessed without ambiguity

## Guidance

Build now, but treat protocol mapping as the next active phase.

That keeps the project moving while reducing the chance of writing the wrong
driver around the wrong mental model.
