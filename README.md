# 🐧 linux-synaptics-hid-fingerprint

> Open-source Linux kernel driver for **Synaptics HID-over-I2C fingerprint sensors**.  
> Targeting the upstream Linux kernel `drivers/hid/` tree.

[![CI](https://github.com/munene/linux-synaptics-hid-fingerprint/actions/workflows/ci.yml/badge.svg)](https://github.com/munene/linux-synaptics-hid-fingerprint/actions)
![Rust](https://img.shields.io/badge/rust-stable-orange)
![License](https://img.shields.io/badge/license-GPL--2.0-blue)
![Status](https://img.shields.io/badge/status-research%20%2F%20WIP-yellow)
![Target](https://img.shields.io/badge/target-Linux%20kernel%20upstream-green)

---

## 🎯 Goal

Get Synaptics HID-over-I2C fingerprint sensors working on Linux — for **every laptop** that uses them, not just one.

These sensors are currently bound to `hid-multitouch` (wrong driver) and show up as `No devices available` in `fprintd`. This project fixes that.

---

## 🖥️ Supported Hardware

See [SUPPORTED_DEVICES.md](./SUPPORTED_DEVICES.md) for the full list.

**Confirmed working:**
- Synaptics SYNA30B8 (`06CB:CE1A`) — HP EliteBook x360 1040 G7

**Likely compatible** (same Vendor ID `06CB`, community testing needed):
- SYNA3097, SYNA305A, SYNA30AF, SYNA7DB5 and other Synaptics I2C sensors

> 💡 **Have a Synaptics fingerprint reader that doesn't work on Linux?**  
> Run `cat /sys/class/hidraw/hidraw*/device/uevent | grep -i syna` and open an issue!

---

## 🗺️ Roadmap

| Phase | Goal | Status |
|---|---|---|
| 1 | Identify sensor & capture HID descriptor | ✅ Done |
| 1 | Set up Rust dev container | ✅ Done |
| 2 | Decode HID report format | 🔄 In progress |
| 2 | Capture fingerprint image data | ⏳ Pending |
| 3 | Userspace Rust driver + libfprint plugin | ⏳ Pending |
| 4 | **Linux kernel C driver submission** | 🎯 End goal |

See [KERNEL_SUBMISSION.md](./KERNEL_SUBMISSION.md) for the full upstream submission checklist.

---

## 🚀 Getting Started

### Prerequisites
- [VS Code](https://code.visualstudio.com/) + [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
- [Docker Desktop](https://www.docker.com/products/docker-desktop/)
- A laptop with a Synaptics HID fingerprint sensor

### Open in Dev Container
```bash
git clone https://github.com/munene/linux-synaptics-hid-fingerprint
cd linux-synaptics-hid-fingerprint
code .
# VS Code: "Reopen in Container" → Yes
```

### Run
```bash
sudo cargo run -- probe      # Detect sensor
sudo cargo run -- listen     # Read raw HID reports (place finger on sensor)
cargo nextest run            # Run tests (no hardware needed)
cargo watch -x check         # Auto-check on file save
```

---

## 🤝 Contributing

**You don't need the hardware to contribute!**

- Protocol logic & decoding → use `MockSensor` in `src/sensor.rs`
- Tests → `tests/mock_sensor_test.rs`
- Docs → always welcome
- Have hardware? → Run `cargo run -- listen` and share the output in an issue

Please follow [Linux kernel commit message format](./KERNEL_SUBMISSION.md#kernel-patch-format) for all commits — we want the git history clean for upstream submission.

---

## 📄 License

GPL-2.0 — same as the Linux kernel.