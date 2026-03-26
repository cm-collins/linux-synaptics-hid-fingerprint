#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VID="${SYNAPTICS_VID:-06cb}"
PID="${SYNAPTICS_PID:-00e9}"
SESSION_ID="$(date -u +%Y%m%dT%H%M%SZ)"
OUTPUT_DIR="${1:-${ROOT_DIR}/captures/phase2-session-${SESSION_ID}}"
BASELINE_DIR="${OUTPUT_DIR}/baseline"
USBMON_DURATION="${SYNAPTICS_USBMON_DURATION:-5}"
USBMON_USE_SUDO="${SYNAPTICS_USBMON_USE_SUDO:-0}"
RUNTIME_ENDPOINT="${SYNAPTICS_RUNTIME_ENDPOINT:-0x83}"
RUNTIME_TRANSPORT="${SYNAPTICS_RUNTIME_TRANSPORT:-interrupt}"
RUNTIME_LENGTH="${SYNAPTICS_RUNTIME_LENGTH:-64}"
RUNTIME_TIMEOUT_MS="${SYNAPTICS_RUNTIME_TIMEOUT_MS:-250}"
RUNTIME_ITERATIONS="${SYNAPTICS_RUNTIME_ITERATIONS:-8}"
RUNTIME_DELAY_MS="${SYNAPTICS_RUNTIME_DELAY_MS:-500}"
SECONDARY_ENDPOINT="${SYNAPTICS_SECONDARY_ENDPOINT:-}"
SECONDARY_TRANSPORT="${SYNAPTICS_SECONDARY_TRANSPORT:-bulk}"
SECONDARY_LENGTH="${SYNAPTICS_SECONDARY_LENGTH:-64}"
SECONDARY_TIMEOUT_MS="${SYNAPTICS_SECONDARY_TIMEOUT_MS:-250}"
SECONDARY_ITERATIONS="${SYNAPTICS_SECONDARY_ITERATIONS:-8}"
SECONDARY_DELAY_MS="${SYNAPTICS_SECONDARY_DELAY_MS:-500}"
SESSION_NOTE="${SYNAPTICS_SESSION_NOTE:-}"
CAPTURE_SYSFS_STATE="${SYNAPTICS_CAPTURE_SYSFS_STATE:-1}"
FORCE_RUNTIME_PM_ON="${SYNAPTICS_FORCE_RUNTIME_PM_ON:-0}"
RUNTIME_PM_USE_SUDO="${SYNAPTICS_RUNTIME_PM_USE_SUDO:-${USBMON_USE_SUDO}}"
RUNTIME_PROBE_STATUS=0
SECONDARY_RUNTIME_PROBE_STATUS=0
USBMON_STATUS=0
USBMON_ANALYSIS_STATUS=0
USBMON_CAPTURE_PATH=""
USBMON_PID=""
RUNTIME_PM_STATE_DIR="${OUTPUT_DIR}/runtime-pm"
RUNTIME_PM_RESTORE_NEEDED=0

estimate_trace_duration_seconds() {
    local timeout_ms="$1"
    local iterations="$2"
    local delay_ms="$3"
    local total_ms

    total_ms=$(( iterations * timeout_ms ))
    if [ "${iterations}" -gt 1 ]; then
        total_ms=$(( total_ms + (iterations - 1) * delay_ms ))
    fi

    total_ms=$(( total_ms + 2000 ))
    echo $(( (total_ms + 999) / 1000 ))
}

PRIMARY_TRACE_DURATION_SECONDS="$(estimate_trace_duration_seconds "${RUNTIME_TIMEOUT_MS}" "${RUNTIME_ITERATIONS}" "${RUNTIME_DELAY_MS}")"
SECONDARY_TRACE_DURATION_SECONDS=0
if [ -n "${SECONDARY_ENDPOINT}" ]; then
    SECONDARY_TRACE_DURATION_SECONDS="$(estimate_trace_duration_seconds "${SECONDARY_TIMEOUT_MS}" "${SECONDARY_ITERATIONS}" "${SECONDARY_DELAY_MS}")"
fi
TRACE_DURATION_SECONDS="$(( PRIMARY_TRACE_DURATION_SECONDS + SECONDARY_TRACE_DURATION_SECONDS ))"
if [ "${USBMON_DURATION}" -lt "${TRACE_DURATION_SECONDS}" ]; then
    EFFECTIVE_USBMON_DURATION="${TRACE_DURATION_SECONDS}"
else
    EFFECTIVE_USBMON_DURATION="${USBMON_DURATION}"
fi

run_runtime_trace() {
    local output_path="$1"
    local stdout_path="$2"
    local stderr_path="$3"
    local endpoint="$4"
    local transport="$5"
    local length="$6"
    local timeout_ms="$7"
    local iterations="$8"
    local delay_ms="$9"

    cargo run --manifest-path "${ROOT_DIR}/Cargo.toml" -- \
        runtime-trace \
        --claim 0 \
        --read-ep "${endpoint}" \
        --transport "${transport}" \
        --length "${length}" \
        --timeout-ms "${timeout_ms}" \
        --iterations "${iterations}" \
        --delay-ms "${delay_ms}" \
        --output "${output_path}" \
        > "${stdout_path}" 2> "${stderr_path}"
}

cleanup() {
    if [ "${RUNTIME_PM_RESTORE_NEEDED}" = "1" ]; then
        SYNAPTICS_RUNTIME_PM_USE_SUDO="${RUNTIME_PM_USE_SUDO}" \
            bash "${ROOT_DIR}/scripts/device-runtime-pm.sh" restore "${RUNTIME_PM_STATE_DIR}" \
            > "${OUTPUT_DIR}/runtime-pm-restore.stdout" 2> "${OUTPUT_DIR}/runtime-pm-restore.stderr" || true
    fi
}

trap cleanup EXIT

mkdir -p "${OUTPUT_DIR}"

{
    echo "session_id: ${SESSION_ID}"
    echo "captured_at: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    echo "device_id: ${VID}:${PID}"
    echo "host: $(hostname)"
    echo "kernel: $(uname -srmo)"
    echo "baseline_dir: ${BASELINE_DIR}"
    echo "usbmon_duration_seconds_requested: ${USBMON_DURATION}"
    echo "usbmon_duration_seconds_effective: ${EFFECTIVE_USBMON_DURATION}"
    echo "usbmon_use_sudo: ${USBMON_USE_SUDO}"
    echo "runtime_endpoint: ${RUNTIME_ENDPOINT}"
    echo "runtime_transport: ${RUNTIME_TRANSPORT}"
    echo "runtime_length: ${RUNTIME_LENGTH}"
    echo "runtime_timeout_ms: ${RUNTIME_TIMEOUT_MS}"
    echo "runtime_iterations: ${RUNTIME_ITERATIONS}"
    echo "runtime_delay_ms: ${RUNTIME_DELAY_MS}"
    echo "runtime_primary_estimated_duration_seconds: ${PRIMARY_TRACE_DURATION_SECONDS}"
    if [ -n "${SECONDARY_ENDPOINT}" ]; then
        echo "runtime_secondary_endpoint: ${SECONDARY_ENDPOINT}"
        echo "runtime_secondary_transport: ${SECONDARY_TRANSPORT}"
        echo "runtime_secondary_length: ${SECONDARY_LENGTH}"
        echo "runtime_secondary_timeout_ms: ${SECONDARY_TIMEOUT_MS}"
        echo "runtime_secondary_iterations: ${SECONDARY_ITERATIONS}"
        echo "runtime_secondary_delay_ms: ${SECONDARY_DELAY_MS}"
        echo "runtime_secondary_estimated_duration_seconds: ${SECONDARY_TRACE_DURATION_SECONDS}"
    fi
    echo "runtime_estimated_duration_seconds: ${TRACE_DURATION_SECONDS}"
    echo "force_runtime_pm_on: ${FORCE_RUNTIME_PM_ON}"
    echo "runtime_pm_use_sudo: ${RUNTIME_PM_USE_SUDO}"
    if [ -n "${SESSION_NOTE}" ]; then
        echo "session_note: ${SESSION_NOTE}"
    fi
} > "${OUTPUT_DIR}/session-metadata.txt"

echo "Phase 2 session"
echo "Output dir : ${OUTPUT_DIR}"
echo "Device     : ${VID}:${PID}"
echo ""

echo "Refreshing baseline artifacts..."
bash "${ROOT_DIR}/scripts/run-local-probe.sh" "${BASELINE_DIR}" \
    > "${OUTPUT_DIR}/baseline.log" 2>&1

if [ -d "${ROOT_DIR}/artifacts/local-probe" ]; then
    echo "Comparing captured baseline with artifacts/local-probe..."
    bash "${ROOT_DIR}/scripts/compare-baseline-runs.sh" \
        "${ROOT_DIR}/artifacts/local-probe" \
        "${BASELINE_DIR}" \
        > "${OUTPUT_DIR}/baseline-compare.txt" 2>&1 || true
fi

if [ "${CAPTURE_SYSFS_STATE}" = "1" ]; then
    bash "${ROOT_DIR}/scripts/capture-sysfs-summary.sh" "${OUTPUT_DIR}/sysfs-before" \
        > "${OUTPUT_DIR}/sysfs-before.stdout" 2> "${OUTPUT_DIR}/sysfs-before.stderr"
fi

if [ "${FORCE_RUNTIME_PM_ON}" = "1" ]; then
    SYNAPTICS_RUNTIME_PM_USE_SUDO="${RUNTIME_PM_USE_SUDO}" \
        bash "${ROOT_DIR}/scripts/device-runtime-pm.sh" force-on "${RUNTIME_PM_STATE_DIR}" \
        > "${OUTPUT_DIR}/runtime-pm-force.stdout" 2> "${OUTPUT_DIR}/runtime-pm-force.stderr"
    RUNTIME_PM_RESTORE_NEEDED=1
fi

echo "Attempting bounded runtime probe..."
echo "Attempting usbmon capture..."
echo "Runtime trace: endpoint ${RUNTIME_ENDPOINT}, transport ${RUNTIME_TRANSPORT}, length ${RUNTIME_LENGTH}, timeout ${RUNTIME_TIMEOUT_MS}ms, iterations ${RUNTIME_ITERATIONS}, delay ${RUNTIME_DELAY_MS}ms"
if [ -n "${SECONDARY_ENDPOINT}" ]; then
    echo "Secondary trace: endpoint ${SECONDARY_ENDPOINT}, transport ${SECONDARY_TRANSPORT}, length ${SECONDARY_LENGTH}, timeout ${SECONDARY_TIMEOUT_MS}ms, iterations ${SECONDARY_ITERATIONS}, delay ${SECONDARY_DELAY_MS}ms"
fi
echo "usbmon capture: requested ${USBMON_DURATION}s, effective ${EFFECTIVE_USBMON_DURATION}s"
if [ -n "${SESSION_NOTE}" ]; then
    echo "Session note : ${SESSION_NOTE}"
fi
SYNAPTICS_USBMON_USE_SUDO="${USBMON_USE_SUDO}" \
    bash "${ROOT_DIR}/scripts/capture-usbmon.sh" "${EFFECTIVE_USBMON_DURATION}" "${OUTPUT_DIR}/usbmon" \
    > "${OUTPUT_DIR}/usbmon.stdout" 2> "${OUTPUT_DIR}/usbmon.stderr" &
USBMON_PID=$!

sleep 1

if run_runtime_trace \
    "${OUTPUT_DIR}/runtime-probe.txt" \
    "${OUTPUT_DIR}/runtime-probe.stdout" \
    "${OUTPUT_DIR}/runtime-probe.stderr" \
    "${RUNTIME_ENDPOINT}" \
    "${RUNTIME_TRANSPORT}" \
    "${RUNTIME_LENGTH}" \
    "${RUNTIME_TIMEOUT_MS}" \
    "${RUNTIME_ITERATIONS}" \
    "${RUNTIME_DELAY_MS}"; then
    RUNTIME_PROBE_STATUS=0
else
    RUNTIME_PROBE_STATUS=$?
fi

if [ -n "${SECONDARY_ENDPOINT}" ]; then
    if run_runtime_trace \
        "${OUTPUT_DIR}/runtime-probe-secondary.txt" \
        "${OUTPUT_DIR}/runtime-probe-secondary.stdout" \
        "${OUTPUT_DIR}/runtime-probe-secondary.stderr" \
        "${SECONDARY_ENDPOINT}" \
        "${SECONDARY_TRANSPORT}" \
        "${SECONDARY_LENGTH}" \
        "${SECONDARY_TIMEOUT_MS}" \
        "${SECONDARY_ITERATIONS}" \
        "${SECONDARY_DELAY_MS}"; then
        SECONDARY_RUNTIME_PROBE_STATUS=0
    else
        SECONDARY_RUNTIME_PROBE_STATUS=$?
    fi
fi

if [ "${CAPTURE_SYSFS_STATE}" = "1" ]; then
    bash "${ROOT_DIR}/scripts/capture-sysfs-summary.sh" "${OUTPUT_DIR}/sysfs-after" \
        > "${OUTPUT_DIR}/sysfs-after.stdout" 2> "${OUTPUT_DIR}/sysfs-after.stderr"
fi

if wait "${USBMON_PID}"; then
    USBMON_STATUS=0
else
    USBMON_STATUS=$?
fi

if [ "${USBMON_STATUS}" -eq 0 ]; then
    USBMON_CAPTURE_PATH="$(find "${OUTPUT_DIR}/usbmon" -maxdepth 1 -name 'usbmon-bus*.txt' | sort | head -n 1)"
    if [ -n "${USBMON_CAPTURE_PATH}" ]; then
        BUS_NUMBER="$(awk -F': ' '$1 == "busnum" { print $2 }' "${OUTPUT_DIR}/usbmon/metadata.txt" | tr -d '[:space:]')"
        DEVICE_ADDRESS="$(awk -F': ' '$1 == "devnum" { print $2 }' "${OUTPUT_DIR}/usbmon/metadata.txt" | tr -d '[:space:]')"
        if cargo run --manifest-path "${ROOT_DIR}/Cargo.toml" -- \
            analyze-usbmon \
            --input "${USBMON_CAPTURE_PATH}" \
            --bus "${BUS_NUMBER}" \
            --device "${DEVICE_ADDRESS}" \
            --output "${OUTPUT_DIR}/usbmon-analysis.md" \
            > "${OUTPUT_DIR}/usbmon-analysis.stdout" 2> "${OUTPUT_DIR}/usbmon-analysis.stderr"; then
            USBMON_ANALYSIS_STATUS=0
        else
            USBMON_ANALYSIS_STATUS=$?
        fi
    fi
fi

{
    echo "session_id: ${SESSION_ID}"
    echo "runtime_probe_status: ${RUNTIME_PROBE_STATUS}"
    if [ -n "${SECONDARY_ENDPOINT}" ]; then
        echo "secondary_runtime_probe_status: ${SECONDARY_RUNTIME_PROBE_STATUS}"
    fi
    echo "usbmon_status: ${USBMON_STATUS}"
    echo "baseline_log: ${OUTPUT_DIR}/baseline.log"
    echo "runtime_probe_stdout: ${OUTPUT_DIR}/runtime-probe.stdout"
    echo "runtime_probe_stderr: ${OUTPUT_DIR}/runtime-probe.stderr"
    echo "usbmon_stdout: ${OUTPUT_DIR}/usbmon.stdout"
    echo "usbmon_stderr: ${OUTPUT_DIR}/usbmon.stderr"
    echo "usbmon_analysis_stdout: ${OUTPUT_DIR}/usbmon-analysis.stdout"
    echo "usbmon_analysis_stderr: ${OUTPUT_DIR}/usbmon-analysis.stderr"
    echo "usbmon_analysis_status: ${USBMON_ANALYSIS_STATUS}"
    echo ""
    echo "Interpretation:"
    if [ "${RUNTIME_PROBE_STATUS}" -eq 0 ]; then
        echo "- runtime probe completed successfully"
    else
        echo "- runtime probe did not complete successfully; inspect runtime-probe.stderr"
    fi

    if [ -n "${SECONDARY_ENDPOINT}" ]; then
        if [ "${SECONDARY_RUNTIME_PROBE_STATUS}" -eq 0 ]; then
            echo "- secondary runtime probe completed successfully"
        else
            echo "- secondary runtime probe did not complete successfully; inspect runtime-probe-secondary.stderr"
        fi
    fi

    if [ "${USBMON_STATUS}" -eq 0 ]; then
        echo "- usbmon capture completed successfully"
    else
        echo "- usbmon capture did not complete successfully; inspect usbmon.stderr"
    fi

    if [ "${USBMON_ANALYSIS_STATUS}" -eq 0 ]; then
        echo "- usbmon analysis completed successfully"
    elif [ "${USBMON_STATUS}" -eq 0 ]; then
        echo "- usbmon analysis did not complete successfully; inspect usbmon-analysis.stderr"
    fi
} > "${OUTPUT_DIR}/summary.txt"

echo ""
echo "Saved:"
echo "  ${OUTPUT_DIR}/session-metadata.txt"
echo "  ${OUTPUT_DIR}/baseline.log"
echo "  ${OUTPUT_DIR}/summary.txt"
echo "  ${BASELINE_DIR}/"
if [ "${CAPTURE_SYSFS_STATE}" = "1" ]; then
    echo "  ${OUTPUT_DIR}/sysfs-before/"
    echo "  ${OUTPUT_DIR}/sysfs-after/"
fi
if [ "${FORCE_RUNTIME_PM_ON}" = "1" ]; then
    echo "  ${OUTPUT_DIR}/runtime-pm-force.stdout"
    echo "  ${OUTPUT_DIR}/runtime-pm-force.stderr"
fi
echo "  ${OUTPUT_DIR}/runtime-probe.stdout"
echo "  ${OUTPUT_DIR}/runtime-probe.stderr"
if [ -n "${SECONDARY_ENDPOINT}" ]; then
    echo "  ${OUTPUT_DIR}/runtime-probe-secondary.stdout"
    echo "  ${OUTPUT_DIR}/runtime-probe-secondary.stderr"
fi
echo "  ${OUTPUT_DIR}/usbmon.stdout"
echo "  ${OUTPUT_DIR}/usbmon.stderr"
echo "  ${OUTPUT_DIR}/usbmon-analysis.stdout"
echo "  ${OUTPUT_DIR}/usbmon-analysis.stderr"
echo "  ${OUTPUT_DIR}/usbmon-analysis.md"
