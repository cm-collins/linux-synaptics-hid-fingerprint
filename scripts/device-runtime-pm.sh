#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VID="${SYNAPTICS_VID:-06cb}"
PID="${SYNAPTICS_PID:-00e9}"
USE_SUDO="${SYNAPTICS_RUNTIME_PM_USE_SUDO:-0}"
COMMAND="${1:-status}"
STATE_DIR="${2:-${ROOT_DIR}/artifacts/runtime-pm-state}"

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

read_file_value() {
    local path="$1"
    if [ -f "${path}" ]; then
        tr -d '\n' < "${path}"
    fi
}

write_file_value() {
    local path="$1"
    local value="$2"

    [ -f "${path}" ] || return 0

    if [ "${USE_SUDO}" = "1" ]; then
        printf '%s\n' "${value}" | sudo tee "${path}" >/dev/null
    else
        printf '%s\n' "${value}" > "${path}"
    fi
}

capture_status() {
    local device_path="$1"
    local interface_path="$2"

    echo "captured_at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    echo "device_path: ${device_path}"
    echo "interface_path: ${interface_path}"

    for file in power/control power/runtime_status power/runtime_enabled power/autosuspend power/autosuspend_delay_ms power/wakeup; do
        if [ -f "${device_path}/${file}" ]; then
            printf 'device/%s: %s\n' "${file}" "$(read_file_value "${device_path}/${file}")"
        fi
    done

    for file in power/control power/runtime_status power/runtime_enabled power/autosuspend power/autosuspend_delay_ms power/wakeup; do
        if [ -f "${interface_path}/${file}" ]; then
            printf 'interface/%s: %s\n' "${file}" "$(read_file_value "${interface_path}/${file}")"
        fi
    done
}

save_state() {
    local path="$1"
    local output="$2"
    if [ -f "${path}" ]; then
        read_file_value "${path}" > "${output}"
    fi
}

restore_state() {
    local state_file="$1"
    local target_path="$2"
    if [ -f "${state_file}" ] && [ -f "${target_path}" ]; then
        write_file_value "${target_path}" "$(cat "${state_file}")"
    fi
}

VID="$(normalize_hex "${VID}")"
PID="$(normalize_hex "${PID}")"
DEVICE_PATH="$(find_device_path "${VID}" "${PID}")" || {
    echo "The target reader is not visible under /sys/bus/usb/devices." >&2
    exit 1
}
INTERFACE_PATH="${DEVICE_PATH}:1.0"

case "${COMMAND}" in
    status)
        capture_status "${DEVICE_PATH}" "${INTERFACE_PATH}"
        ;;
    force-on)
        mkdir -p "${STATE_DIR}"
        save_state "${DEVICE_PATH}/power/control" "${STATE_DIR}/device_power_control.txt"
        save_state "${INTERFACE_PATH}/power/control" "${STATE_DIR}/interface_power_control.txt"
        capture_status "${DEVICE_PATH}" "${INTERFACE_PATH}" > "${STATE_DIR}/before.txt"
        write_file_value "${DEVICE_PATH}/power/control" "on"
        write_file_value "${INTERFACE_PATH}/power/control" "on"
        capture_status "${DEVICE_PATH}" "${INTERFACE_PATH}" > "${STATE_DIR}/after-force-on.txt"
        echo "Saved runtime PM state:"
        echo "  ${STATE_DIR}/before.txt"
        echo "  ${STATE_DIR}/after-force-on.txt"
        ;;
    restore)
        restore_state "${STATE_DIR}/device_power_control.txt" "${DEVICE_PATH}/power/control"
        restore_state "${STATE_DIR}/interface_power_control.txt" "${INTERFACE_PATH}/power/control"
        capture_status "${DEVICE_PATH}" "${INTERFACE_PATH}" > "${STATE_DIR}/after-restore.txt"
        echo "Restored runtime PM state:"
        echo "  ${STATE_DIR}/after-restore.txt"
        ;;
    *)
        echo "Unknown command: ${COMMAND}" >&2
        echo "Usage: $0 <status|force-on|restore> [state-dir]" >&2
        exit 1
        ;;
esac
