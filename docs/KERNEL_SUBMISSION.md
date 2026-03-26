# Linux Kernel Submission Guide

> Roadmap for getting this driver merged into the official Linux kernel tree.
> Target: `drivers/input/fingerprint/` or `drivers/hid/`

---

## 📋 Kernel Submission Checklist

### Phase 1 — Research (Current)
- [ ] Capture full HID report descriptor ✅
- [ ] Identify correct hidraw device node ✅
- [ ] Decode HID report format
- [ ] Capture fingerprint image data
- [ ] Document the protocol

### Phase 2 — Userspace Prototype (Rust)
- [ ] Working userspace driver reads fingerprint images
- [ ] Integrate with libfprint as a plugin
- [ ] Test enroll + verify flow
- [ ] Document all findings

### Phase 3 — Kernel Driver (C or Rust)
- [ ] Write kernel driver following `Documentation/process/coding-style.rst`
- [ ] Add `Kconfig` entry under `drivers/hid/`
- [ ] Add `Makefile` entry
- [ ] Write device tree binding YAML
- [ ] Add entry to `MAINTAINERS`
- [ ] Test with `checkpatch.pl`
- [ ] Test with `sparse` and `smatch`

### Phase 4 — Patch Submission
- [ ] Subscribe to `linux-input@vger.kernel.org`
- [ ] Send RFC (Request for Comments) patch series
- [ ] Address maintainer feedback
- [ ] Send v2, v3... until accepted
- [ ] Merged into `input-next` tree → Linux mainline 🎉

---

## 📧 Kernel Mailing Lists

| List | Purpose |
|---|---|
| `linux-input@vger.kernel.org` | Primary — input/HID drivers |
| `linux-kernel@vger.kernel.org` | CC for final submission |
| `linux-usb@vger.kernel.org` | CC if USB fallback added |

### Relevant Maintainers to CC
```
Jiri Kosina <jikos@kernel.org>          # HID subsystem
Benjamin Tissoires <bentiss@kernel.org> # HID subsystem
Dmitry Torokhov <dmitry.t@samsung.com> # Input subsystem
```

---

## 🛠️ Kernel Patch Format

Every commit destined for the kernel must follow this format:

```
hid: synaptics: add driver for HID-over-I2C fingerprint sensors

Add support for Synaptics HID-over-I2C fingerprint sensors,
starting with SYNA30B8 (VID:06CB PID:CE1A) as found in the
HP EliteBook x360 1040 G7.

These sensors are currently bound to hid-multitouch which is
incorrect. This driver correctly handles the fingerprint HID
report descriptor and exposes the sensor to userspace via
the standard /dev/hidraw interface for libfprint integration.

Tested-by: Munene <your-real@email.com>
Signed-off-by: Munene <your-real@email.com>
```

---

## 🧹 Kernel Code Requirements

```bash
# Check patch style before submitting
./scripts/checkpatch.pl --strict your-patch.patch

# Static analysis
make C=1 drivers/hid/hid-synaptics-fingerprint.o

# Sparse analysis
make C=2 drivers/hid/hid-synaptics-fingerprint.o
```

---

## 📁 Target Kernel Directory Structure

```
linux/
├── drivers/
│   └── hid/
│       ├── Kconfig                    ← Add: config HID_SYNAPTICS_FINGERPRINT
│       ├── Makefile                   ← Add: obj-$(CONFIG_HID_SYNAPTICS_FINGERPRINT)
│       └── hid-synaptics-fingerprint.c  ← The driver
├── Documentation/
│   └── devicetree/bindings/
│       └── input/fingerprint/
│           └── synaptics,syna30b8.yaml
└── MAINTAINERS                        ← Add entry
```

---

## 🔗 Resources

- [Submitting Patches](https://www.kernel.org/doc/html/latest/process/submitting-patches.html)
- [HID Driver Guide](https://www.kernel.org/doc/html/latest/hid/hid-transport.html)
- [Kernel Coding Style](https://www.kernel.org/doc/html/latest/process/coding-style.html)
- [libfprint Driver Guide](https://fprint.freedesktop.org/libfprint-dev/writing-drivers.html)
- [Existing HID drivers](https://github.com/torvalds/linux/tree/master/drivers/hid)