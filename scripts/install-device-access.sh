#!/usr/bin/env bash
set -euo pipefail

RULE_PATH="/etc/udev/rules.d/99-synaptics-06cb-00e9.rules"
RULE='SUBSYSTEM=="usb", ATTR{idVendor}=="06cb", ATTR{idProduct}=="00e9", MODE="0660", GROUP="plugdev", TAG+="uaccess"'
ACTION="${1:-print}"

print_rule() {
    echo "${RULE}"
}

case "${ACTION}" in
    print)
        print_rule
        ;;
    install)
        if [ "$(id -u)" -ne 0 ]; then
            echo "Re-run with sudo to install the udev rule:" >&2
            echo "  sudo $0 install" >&2
            exit 1
        fi

        printf '%s\n' "${RULE}" > "${RULE_PATH}"
        udevadm control --reload-rules
        udevadm trigger --attr-match=idVendor=06cb --attr-match=idProduct=00e9
        echo "Installed ${RULE_PATH}"
        ;;
    *)
        echo "Usage: $0 [print|install]" >&2
        exit 1
        ;;
esac
