#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEFT_DIR="${1:-${ROOT_DIR}/artifacts/local-probe}"
RIGHT_DIR="${2:-}"

if [ -z "${RIGHT_DIR}" ]; then
    echo "Usage: $0 <baseline-dir> <replay-dir>" >&2
    exit 1
fi

FILES=(
    lsusb.txt
    usb-devices.txt
    sysfs-device.txt
    sysfs-interface.txt
    probe.txt
    runtime-probe.txt
)

differences_found=0

normalized_diff() {
    local left_path="$1"
    local right_path="$2"

    diff -u \
        <(grep -v '^captured_at: ' "${left_path}") \
        <(grep -v '^captured_at: ' "${right_path}")
}

for name in "${FILES[@]}"; do
    left_path="${LEFT_DIR}/${name}"
    right_path="${RIGHT_DIR}/${name}"

    if [ ! -e "${left_path}" ] && [ ! -e "${right_path}" ]; then
        continue
    fi

    echo "== ${name} =="

    if [ ! -e "${left_path}" ]; then
        echo "missing in baseline: ${left_path}"
        differences_found=1
        echo ""
        continue
    fi

    if [ ! -e "${right_path}" ]; then
        echo "missing in replay: ${right_path}"
        differences_found=1
        echo ""
        continue
    fi

    if normalized_diff "${left_path}" "${right_path}"; then
        echo "no differences"
    else
        differences_found=1
    fi

    echo ""
done

if [ "${differences_found}" -eq 0 ]; then
    echo "Baseline runs are comparable."
    exit 0
fi

echo "Differences detected between baseline runs."
exit 1
