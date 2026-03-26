#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VID="${SYNAPTICS_VID:-06cb}"
PID="${SYNAPTICS_PID:-00e9}"
OUTPUT_DIR="${1:-${ROOT_DIR}/artifacts/local-probe}"

normalize_hex() {
    printf '%s' "${1#0x}" | tr '[:upper:]' '[:lower:]'
}

find_device_path() {
    local vid="$1"
    local pid="$2"
    local path

    for path in /sys/bus/usb/devices/*; do
        [ -f "${path}/idVendor" ] || continue
        [ -f "${path}/idProduct" ] || continue

        if [ "$(tr '[:upper:]' '[:lower:]' < "${path}/idVendor")" = "${vid}" ] \
            && [ "$(tr '[:upper:]' '[:lower:]' < "${path}/idProduct")" = "${pid}" ]; then
            printf '%s\n' "${path}"
            return 0
        fi
    done

    return 1
}

VID="$(normalize_hex "${VID}")"
PID="$(normalize_hex "${PID}")"

mkdir -p "${OUTPUT_DIR}"

DEVICE_PATH="$(find_device_path "${VID}" "${PID}")" || {
    echo "The target reader is not visible under /sys/bus/usb/devices." >&2
    exit 1
}

INTERFACE_PATH="${DEVICE_PATH}:1.0"
CAPTURED_AT="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

{
    echo "captured_at: ${CAPTURED_AT}"
    echo "sysfs_path: ${DEVICE_PATH}"

    for file in idVendor idProduct busnum devnum speed version bConfigurationValue bNumInterfaces authorized removable serial; do
        if [ -f "${DEVICE_PATH}/${file}" ]; then
            printf '%s: ' "${file}"
            tr -d '\n' < "${DEVICE_PATH}/${file}"
            printf '\n'
        fi
    done

    for file in power/control power/runtime_status power/runtime_enabled power/autosuspend power/autosuspend_delay_ms power/wakeup; do
        if [ -f "${DEVICE_PATH}/${file}" ]; then
            printf '%s: ' "${file}"
            tr -d '\n' < "${DEVICE_PATH}/${file}"
            printf '\n'
        fi
    done
} > "${OUTPUT_DIR}/sysfs-device.txt"

{
    echo "captured_at: ${CAPTURED_AT}"
    echo "interface_path: ${INTERFACE_PATH}"

    for file in bInterfaceClass bInterfaceSubClass bInterfaceProtocol supports_autosuspend; do
        if [ -f "${INTERFACE_PATH}/${file}" ]; then
            printf '%s: ' "${file}"
            tr -d '\n' < "${INTERFACE_PATH}/${file}"
            printf '\n'
        fi
    done

    for file in power/control power/runtime_status power/runtime_enabled power/autosuspend power/autosuspend_delay_ms power/wakeup; do
        if [ -f "${INTERFACE_PATH}/${file}" ]; then
            printf '%s: ' "${file}"
            tr -d '\n' < "${INTERFACE_PATH}/${file}"
            printf '\n'
        fi
    done

    if [ -L "${INTERFACE_PATH}/driver" ]; then
        echo "driver: $(basename "$(readlink -f "${INTERFACE_PATH}/driver")")"
    else
        echo "driver: none"
    fi
} > "${OUTPUT_DIR}/sysfs-interface.txt"

echo "Saved sysfs summaries:"
echo "  ${OUTPUT_DIR}/sysfs-device.txt"
echo "  ${OUTPUT_DIR}/sysfs-interface.txt"
