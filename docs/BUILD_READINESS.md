# Build Readiness

This document answers a practical question:

Are the repository docs strong enough to start building?

The answer is yes, but only if "building" starts with instrumentation and
evidence capture, not with a final driver.

## Current Readiness

The repository is ready for:

- development container setup
- USB device enumeration
- baseline descriptor collection
- experiment planning
- creation of the first userspace probing tools

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
- explicit non-goals that keep scope controlled

## What Is Still Missing

The repo still lacks several grounding artifacts that should exist before major
implementation work:

- a checked-in device profile with descriptor facts and endpoint inventory
- an evidence ledger that records what was observed, when, and how
- a clear split between confirmed facts, hypotheses, and unknowns
- an experiment journal for non-destructive probing
- a real Rust workspace for instrumentation

## Recommended Next Build Target

The first implementation target should be a small Rust tool focused on safe
inspection, not a biometric workflow.

That tool should be able to:

- list the target device and interface descriptors
- claim the interface safely when permitted
- print endpoint inventory in a stable format
- perform bounded bulk and interrupt reads with timeouts
- log enough detail to compare runs across sessions

## Decision Gates Before Deeper Driver Work

Before protocol-specific work accelerates, the project should have evidence for:

- exact interface numbers and endpoint addresses
- startup behavior after interface claim
- whether interrupt traffic appears while idle
- whether device responses are deterministic across cold and warm boots
- whether any command framing can be identified without destructive probing

## Definition Of "Grounded Enough To Build"

The project is grounded enough to move beyond instrumentation when all of the
following are true:

- the target device profile is stored in the repo
- capture commands produce reproducible artifacts
- assumptions are tracked explicitly
- the first probing tool can reproduce the same baseline facts on demand
- the team can explain what is known versus guessed without ambiguity

## Guidance

Build now, but build the fact-gathering layer first.

That keeps the project moving while reducing the chance of writing the wrong
driver around the wrong mental model.
