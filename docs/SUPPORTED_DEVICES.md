# Supported & Tested Devices

This driver targets **all Synaptics HID-over-I2C fingerprint sensors**.
If your device works, please open a PR to add it to this list.

## ✅ Confirmed Working

| Sensor ID | VID:PID | Laptop | Contributor | Status |
|---|---|---|---|---|
| SYNA30B8 | 06CB:CE1A | HP EliteBook x360 1040 G7 | @munene | Research / WIP |

---

## 🔍 Known Synaptics HID Sensors (Unconfirmed)

These share the same `06CB` Vendor ID and likely use a similar HID protocol.
Community testing needed:

| Sensor ID | VID:PID | Known Laptops |
|---|---|---|
| SYNA3097 | 06CB:0097 | Lenovo ThinkPad X1 Carbon (various) |
| SYNA305A | 06CB:005A | Dell Latitude 5000 series |
| SYNA3255 | 06CB:* | HP ProBook 450 G6 |
| SYNA7DB5 | 06CB:7DB5 | Lenovo IdeaPad |
| SYNA30AF | 06CB:00AF | HP EliteBook 840 G6 |

> **Have one of these?** Run the probe tool and open an issue with your output!

---

## 🧪 How to Test Your Device

```bash
# 1. Check if your sensor is Synaptics HID
cat /sys/bus/i2c/devices/i2c-SYNA*/uevent

# 2. Get VID:PID
cat /sys/class/hidraw/hidraw*/device/uevent | grep -i syna

# 3. Run the probe tool
sudo cargo run -- probe

# 4. Open an issue with your output at:
# https://github.com/munene/linux-synaptics-hid-fingerprint/issues/new
```

---

## ❌ Out of Scope

- Synaptics **USB** fingerprint readers (different protocol)
- Non-Synaptics sensors (Goodix, ELAN, Validity — separate projects)
- Windows Hello — Linux only