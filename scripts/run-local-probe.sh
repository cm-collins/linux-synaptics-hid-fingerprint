#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VID="${SYNAPTICS_VID:-06cb}"
PID="${SYNAPTICS_PID:-00e9}"
OUTPUT_DIR="${1:-${ROOT_DIR}/artifacts/local-probe}"
DEVICE_ID="${VID}:${PID}"

require_command() {
    local command_name="$1"

    if command -v "${command_name}" >/dev/null 2>&1; then
        return 0
    fi

    echo "Missing required command: ${command_name}" >&2
    exit 1
}

require_command cargo
require_command lsusb

mkdir -p "${OUTPUT_DIR}" "${ROOT_DIR}/notes"

echo "Local Synaptics probe"
echo "Repository : ${ROOT_DIR}"
echo "USB device : ${DEVICE_ID}"
echo "Output dir : ${OUTPUT_DIR}"
echo ""

if ! lsusb -d "${DEVICE_ID}" >/dev/null 2>&1; then
    echo "The target reader is not visible to the local machine." >&2
    echo "Check that the fingerprint reader is attached and retry:" >&2
    echo "  lsusb -d ${DEVICE_ID}" >&2
    exit 1
fi

echo "Saving baseline USB facts..."
lsusb -d "${DEVICE_ID}" | tee "${OUTPUT_DIR}/lsusb.txt"

if command -v usb-devices >/dev/null 2>&1; then
    usb-devices | sed -n "/Vendor=${VID} ProdID=${PID}/,+20p" | tee "${OUTPUT_DIR}/usb-devices.txt"
else
    echo "usb-devices not found; skipping usb-devices capture" | tee "${OUTPUT_DIR}/usb-devices.txt"
fi

echo ""
echo "Saving sysfs metadata..."
bash "${ROOT_DIR}/scripts/capture-sysfs-summary.sh" "${OUTPUT_DIR}"

{
    echo "captured_at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    echo "device_id: ${DEVICE_ID}"
    echo "host: $(hostname)"
    echo "kernel: $(uname -srmo)"
    echo "script: scripts/run-local-probe.sh"
} > "${OUTPUT_DIR}/manifest.txt"

echo ""
echo "Running Rust probe..."
cargo run --manifest-path "${ROOT_DIR}/Cargo.toml" -- probe --output "${OUTPUT_DIR}/probe.txt"

echo ""
echo "Writing Markdown device profile..."
cargo run --manifest-path "${ROOT_DIR}/Cargo.toml" -- device-profile --output "${ROOT_DIR}/notes/device-profile.md"

if [ "${SYNAPTICS_RUNTIME_PROBE:-0}" = "1" ]; then
    echo ""
    echo "Running optional bounded runtime probe..."
    cargo run --manifest-path "${ROOT_DIR}/Cargo.toml" -- \
        probe \
        --claim 0 \
        --read-ep 0x83 \
        --length 64 \
        --timeout-ms 250 \
        --output "${OUTPUT_DIR}/runtime-probe.txt"
fi

echo ""
echo "Saved:"
echo "  ${OUTPUT_DIR}/lsusb.txt"
echo "  ${OUTPUT_DIR}/usb-devices.txt"
echo "  ${OUTPUT_DIR}/sysfs-device.txt"
echo "  ${OUTPUT_DIR}/sysfs-interface.txt"
echo "  ${OUTPUT_DIR}/manifest.txt"
echo "  ${OUTPUT_DIR}/probe.txt"
echo "  ${ROOT_DIR}/notes/device-profile.md"

if [ "${SYNAPTICS_RUNTIME_PROBE:-0}" = "1" ]; then
    echo "  ${OUTPUT_DIR}/runtime-probe.txt"
fi
