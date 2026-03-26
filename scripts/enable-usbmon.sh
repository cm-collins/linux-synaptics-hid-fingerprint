#!/usr/bin/env bash
set -euo pipefail

ACTION="${1:-status}"

show_status() {
    if ls /sys/kernel/debug/usb/usbmon/*u >/dev/null 2>&1; then
        echo "usbmon: available"
        ls /sys/kernel/debug/usb/usbmon/*u
    else
        echo "usbmon: not visible"
    fi
}

case "${ACTION}" in
    status)
        show_status
        ;;
    apply)
        if [ "$(id -u)" -ne 0 ]; then
            echo "Re-run with sudo to enable usbmon on the host:" >&2
            echo "  sudo $0 apply" >&2
            exit 1
        fi

        modprobe usbmon
        if ! mountpoint -q /sys/kernel/debug; then
            mount -t debugfs none /sys/kernel/debug
        fi
        show_status
        ;;
    *)
        echo "Usage: $0 [status|apply]" >&2
        exit 1
        ;;
esac
