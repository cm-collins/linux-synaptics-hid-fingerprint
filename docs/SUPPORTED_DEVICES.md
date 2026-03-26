# Supported Devices

This repository is currently focused on one concrete reverse-engineering target,
not on broad support claims.

## Primary Focus Device

| USB ID | Reader | Laptop | Status |
|---|---|---|---|
| `06CB:00E9` | Synaptics FS7604 Touch Fingerprint Sensor with PurePrint | HP EliteBook x360 1040 G7 | Grounding / active research |

Notes:

- This is the main device the repo is organized around today.
- `lsusb` can see it on the target laptop.
- `fprintd` does not support it yet in the current environment.

## Candidate Related Devices

These are nearby Synaptics USB fingerprint readers worth revisiting once the
`00E9` protocol shape is better understood.

| USB ID | Known Laptop Families | Notes |
|---|---|---|
| `06CB:00B7` | HP EliteBook 840 G6 and related HP G6 systems | Candidate follow-on target |
| `06CB:00F0` | HP EliteBook 840 G8 / 845 / 865 families | Likely related PurePrint family |
| `06CB:00BD` | Lenovo ThinkPad X1 Extreme | Candidate follow-on target |
| `06CB:00FC` | Lenovo ThinkPad X1 Carbon Gen 9/11 | Candidate follow-on target |
| `06CB:009A` | Lenovo ThinkPad X1 Carbon Gen 6 | Older candidate reader family |

These are not currently marked supported. They are research targets only.

## How To Identify Your Reader

Use USB-based inspection first:

```bash
lsusb | grep -i synaptics
usb-devices | sed -n '/Vendor=06cb/,+20p'
```

If you find a Synaptics fingerprint reader, record:

- vendor and product ID
- laptop model
- whether `fprintd-enroll` can see it
- whether the device exposes a driver already

## Current Scope

In scope:

- Synaptics USB fingerprint readers related to the `06CB:00E9` target
- userspace protocol research
- `libfprint` and `fprintd` integration

Out of scope for now:

- the older Synaptics touchpad-style IDs previously listed here
- generic HID/I2C fingerprint claims without proof
- non-Synaptics readers
