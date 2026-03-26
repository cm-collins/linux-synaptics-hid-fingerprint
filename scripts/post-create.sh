#!/usr/bin/env bash
set -euo pipefail

echo "============================================================"
echo " Synaptics 06CB:00E9 dev container setup"
echo " Target: HP EliteBook x360 1040 G7"
echo " Focus : userspace USB reverse engineering and libfprint path"
echo "============================================================"
echo ""

echo "Toolchain"
rustc --version
cargo --version
python3 --version
echo ""

echo "Optional extras"
echo "- run: bash scripts/install-cargo-devtools.sh"
echo "- installs cargo-watch, cargo-expand, cargo-audit, cargo-edit, cargo-outdated, cargo-nextest"
echo "- cargo tree is already built into Cargo"
echo ""

echo "Libraries"
pkg-config --modversion libusb-1.0 || true
pkg-config --modversion libfprint-2 || true
echo ""

mkdir -p captures notes artifacts

echo "Created workspace folders:"
echo "- captures/"
echo "- notes/"
echo "- artifacts/"
echo ""

echo "Next:"
echo "- reopen or start the container with the USB device attached"
echo "- run: lsusb -d 06cb:00e9"
echo "- run: usb-devices | sed -n '/Vendor=06cb ProdID=00e9/,+20p'"
