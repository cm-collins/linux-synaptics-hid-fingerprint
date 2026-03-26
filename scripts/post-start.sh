#!/usr/bin/env bash
# ============================================================
# post-start.sh — Runs every time the container starts
# ============================================================
set -euo pipefail

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${CYAN}🦀 SYNA30B8 Dev Container starting...${NC}"

# ── Check sensor access ───────────────────────────────────
echo -e "\n${YELLOW}🔍 Checking sensor access...${NC}"

if [ -e /dev/hidraw0 ]; then
    echo -e "${GREEN}✅ /dev/hidraw0 is accessible${NC}"
    ls -la /dev/hidraw0
else
    echo -e "${RED}⚠️  /dev/hidraw0 not found!"
    echo "   Make sure to run the container with:"
    echo "   --device=/dev/hidraw0"
    echo -e "   Check Dockerfile.dev runArgs${NC}"
fi

# ── Check sysfs mount ─────────────────────────────────────
if [ -d /sys/bus/i2c/devices/i2c-SYNA30B8:00 ]; then
    echo -e "${GREEN}✅ Sensor sysfs path mounted${NC}"
    cat /sys/bus/i2c/devices/i2c-SYNA30B8:00/uevent 2>/dev/null || true
else
    echo -e "${YELLOW}⚠️  Sensor sysfs not mounted (OK for CI)${NC}"
fi

# ── Rust env check ────────────────────────────────────────
echo -e "\n${YELLOW}🦀 Rust environment:${NC}"
rustc --version
cargo --version

echo -e "\n${GREEN}✅ Container ready! Happy hacking! 🦀${NC}\n"