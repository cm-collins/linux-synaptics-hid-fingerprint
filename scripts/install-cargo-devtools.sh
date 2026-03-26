#!/usr/bin/env bash
set -euo pipefail

tools=(
    cargo-watch
    cargo-expand
    cargo-audit
    cargo-edit
    cargo-outdated
    cargo-nextest
)

export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

echo "Installing optional cargo dev tools with CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}"
echo "This is intentionally not part of the image build."
echo ""

for tool in "${tools[@]}"; do
    if command -v "$tool" >/dev/null 2>&1; then
        echo "Skipping ${tool}; already installed"
        continue
    fi

    echo "Installing ${tool}..."
    cargo install --locked --jobs "${CARGO_BUILD_JOBS}" "${tool}"
done

echo ""
echo "Done."
echo "Note: cargo tree is built into Cargo and does not need a separate install."
