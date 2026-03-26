#!/usr/bin/env bash
set -euo pipefail

echo "Synaptics 06CB:00E9 container startup check"
echo ""

if [ -d /dev/bus/usb ]; then
    echo "USB bus mount: OK"
else
    echo "USB bus mount: missing"
fi

if lsusb -d 06cb:00e9 >/dev/null 2>&1; then
    echo "Target device: detected"
    lsusb -d 06cb:00e9
else
    echo "Target device: not detected"
fi

if [ -d /sys/bus/usb/devices ]; then
    echo "USB sysfs mount: OK"
else
    echo "USB sysfs mount: missing"
fi

if [ -e /sys/kernel/debug/usb/usbmon/0u ] || [ -e /sys/kernel/debug/usb/usbmon/1u ]; then
    echo "usbmon: available"
else
    echo "usbmon: not visible yet"
fi

echo ""
echo "Suggested commands:"
echo "  lsusb -d 06cb:00e9"
echo "  usb-devices | sed -n '/Vendor=06cb ProdID=00e9/,+20p'"
echo "  fprintd-enroll -f right-index-finger"
