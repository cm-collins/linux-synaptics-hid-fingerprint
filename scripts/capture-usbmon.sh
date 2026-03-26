#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VID="${SYNAPTICS_VID:-06cb}"
PID="${SYNAPTICS_PID:-00e9}"
DURATION_SECONDS="${1:-5}"
DEFAULT_OUTPUT_DIR="${ROOT_DIR}/captures/usbmon-$(date -u +%Y%m%dT%H%M%SZ)"
OUTPUT_DIR="${2:-${DEFAULT_OUTPUT_DIR}}"

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
DEVICE_PATH="$(find_device_path "${VID}" "${PID}")" || {
    echo "The target reader is not visible under /sys/bus/usb/devices." >&2
    exit 1
}

BUSNUM="$(tr -d '[:space:]' < "${DEVICE_PATH}/busnum")"
DEVNUM="$(tr -d '[:space:]' < "${DEVICE_PATH}/devnum")"
USBMON_PATH="/sys/kernel/debug/usb/usbmon/${BUSNUM}u"

mkdir -p "${OUTPUT_DIR}"

if [ ! -e "${USBMON_PATH}" ]; then
    echo "usbmon is not visible at ${USBMON_PATH}." >&2
    echo "Enable it on the host first, then retry:" >&2
    echo "  sudo modprobe usbmon" >&2
    echo "  sudo mount -t debugfs none /sys/kernel/debug" >&2
    exit 1
fi

if [ ! -r "${USBMON_PATH}" ]; then
    echo "usbmon exists but is not readable at ${USBMON_PATH}." >&2
    echo "Retry the capture with host permissions if needed." >&2
    exit 1
fi

CAPTURED_AT="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
RAW_CAPTURE_PATH="${OUTPUT_DIR}/usbmon-bus${BUSNUM}.txt"

{
    echo "captured_at: ${CAPTURED_AT}"
    echo "duration_seconds: ${DURATION_SECONDS}"
    echo "device_path: ${DEVICE_PATH}"
    echo "busnum: ${BUSNUM}"
    echo "devnum: ${DEVNUM}"
    echo "usbmon_path: ${USBMON_PATH}"
    echo "note: usbmon text output is bus-scoped; use the bus/device metadata above when filtering or reviewing traces."
} > "${OUTPUT_DIR}/metadata.txt"

echo "Capturing ${DURATION_SECONDS}s of usbmon traffic from bus ${BUSNUM}..."
capture_status=0
timeout "${DURATION_SECONDS}" cat "${USBMON_PATH}" > "${RAW_CAPTURE_PATH}" || capture_status=$?

if [ "${capture_status}" -ne 0 ] && [ "${capture_status}" -ne 124 ]; then
    echo "usbmon capture failed with status ${capture_status}." >&2
    exit "${capture_status}"
fi

echo "Saved:"
echo "  ${OUTPUT_DIR}/metadata.txt"
echo "  ${RAW_CAPTURE_PATH}"
