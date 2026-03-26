#!/usr/bin/env bash
# ============================================================
# setup-repo.sh — Create & configure GitHub repository
# Run this ONCE from inside the dev container
# ============================================================
set -euo pipefail

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BOLD='\033[1m'
NC='\033[0m'

REPO_NAME="linux-synaptics-hid-fingerprint"
REPO_DESC="Linux kernel driver for Synaptics HID-over-I2C fingerprint sensors (SYNA30B8 and family) — targeting upstream Linux kernel drivers tree"
REPO_TOPICS="linux,kernel,fingerprint,driver,rust,synaptics,hid,i2c,libfprint,open-source"

echo -e "${CYAN}${BOLD}"
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  🐧  linux-synaptics-hid-fingerprint                     ║"
echo "║      GitHub Repository Setup                             ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo -e "${NC}"

# ── Check gh auth ─────────────────────────────────────────
echo -e "${YELLOW}🔐 Checking GitHub authentication...${NC}"
if ! gh auth status &>/dev/null; then
  echo -e "${RED}Not authenticated. Logging in...${NC}"
  gh auth login
fi
echo -e "${GREEN}✅ Authenticated as: $(gh api user --jq .login)${NC}\n"

# ── Create the repository ─────────────────────────────────
echo -e "${YELLOW}📦 Creating repository: ${REPO_NAME}...${NC}"
gh repo create "$REPO_NAME" \
  --public \
  --description "$REPO_DESC" \
  --license gpl-2.0 \
  --gitignore Rust \
  --clone

cd "$REPO_NAME"
echo -e "${GREEN}✅ Repository created and cloned${NC}\n"

# ── Set repository topics ─────────────────────────────────
echo -e "${YELLOW}🏷️  Setting topics...${NC}"
gh repo edit \
  --add-topic linux \
  --add-topic kernel \
  --add-topic fingerprint \
  --add-topic driver \
  --add-topic rust \
  --add-topic synaptics \
  --add-topic hid \
  --add-topic i2c \
  --add-topic libfprint \
  --add-topic open-source
echo -e "${GREEN}✅ Topics set${NC}\n"

# ── Configure branch protection on main ──────────────────
echo -e "${YELLOW}🔒 Setting up branch protection...${NC}"
gh api \
  --method PUT \
  -H "Accept: application/vnd.github+json" \
  /repos/$(gh api user --jq .login)/$REPO_NAME/branches/main/protection \
  -f required_status_checks='{"strict":true,"contexts":["Check & Clippy","Tests (Mock Sensor)"]}' \
  -f enforce_admins=false \
  -f required_pull_request_reviews='{"required_approving_review_count":1}' \
  -f restrictions=null \
  2>/dev/null || echo -e "${YELLOW}⚠️  Branch protection requires GitHub Pro — skipping${NC}"

# ── Create branch structure ───────────────────────────────
echo -e "${YELLOW}🌿 Setting up branches...${NC}"
git checkout -b develop
git push origin develop

git checkout -b feature/syna30b8-initial-probe
git push origin feature/syna30b8-initial-probe

git checkout main
echo -e "${GREEN}✅ Branches: main, develop, feature/syna30b8-initial-probe${NC}\n"

# ── Create GitHub labels for kernel workflow ──────────────
echo -e "${YELLOW}🏷️  Creating issue labels...${NC}"
gh label create "kernel-submission"  --color "0075ca" --description "Related to upstream kernel patch submission"  2>/dev/null || true
gh label create "sensor-research"    --color "e4e669" --description "Reverse engineering & protocol analysis"      2>/dev/null || true
gh label create "rust"               --color "f74c00" --description "Rust implementation"                          2>/dev/null || true
gh label create "hid-protocol"       --color "d93f0b" --description "HID over I2C protocol work"                  2>/dev/null || true
gh label create "needs-hardware"     --color "c5def5" --description "Requires physical Synaptics sensor to test"  2>/dev/null || true
gh label create "hardware-support"   --color "bfdadc" --description "New device/sensor support"                   2>/dev/null || true
gh label create "libfprint"          --color "1d76db" --description "libfprint integration"                       2>/dev/null || true
echo -e "${GREEN}✅ Labels created${NC}\n"

# ── Create first milestone ────────────────────────────────
echo -e "${YELLOW}🎯 Creating milestones...${NC}"
gh api \
  --method POST \
  /repos/$(gh api user --jq .login)/$REPO_NAME/milestones \
  -f title="Phase 1: Protocol Research" \
  -f description="Reverse engineer SYNA30B8 HID protocol, capture and decode reports" \
  -f state="open" 2>/dev/null || true

gh api \
  --method POST \
  /repos/$(gh api user --jq .login)/$REPO_NAME/milestones \
  -f title="Phase 2: Userspace Driver" \
  -f description="Working Rust userspace driver that can capture fingerprint images" \
  -f state="open" 2>/dev/null || true

gh api \
  --method POST \
  /repos/$(gh api user --jq .linux)/$REPO_NAME/milestones \
  -f title="Phase 3: Kernel Submission" \
  -f description="Prepare and submit patch series to Linux kernel drivers/input/fingerprint" \
  -f state="open" 2>/dev/null || true
echo -e "${GREEN}✅ Milestones created${NC}\n"

# ── First commit with project structure ──────────────────
echo -e "${YELLOW}📝 Creating initial commit...${NC}"
cp -r /workspace/* . 2>/dev/null || true

git add -A
git commit -m "feat: initial project scaffold

linux-synaptics-hid-fingerprint: Rust driver for Synaptics
HID-over-I2C fingerprint sensors targeting Linux kernel upstream.

Hardware confirmed:
- Synaptics SYNA30B8 (VID:06CB PID:CE1A)
- HP EliteBook x360 1040 G7
- Bus: HID over I2C (i2c_hid_acpi)
- HID report descriptor captured and attached

Signed-off-by: Munene <your@email.com>"

git push origin main
echo -e "${GREEN}✅ Initial commit pushed${NC}\n"

# ── Summary ───────────────────────────────────────────────
REPO_URL=$(gh repo view --json url --jq .url)
echo -e "${GREEN}${BOLD}"
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  🎉 Repository ready!                                    ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo -e "${NC}"
echo "  🔗 URL      : $REPO_URL"
echo "  🌿 Branches : main, develop, feature/syna30b8-initial-probe"
echo "  🎯 Goal     : Linux kernel drivers/input/fingerprint tree"
echo ""
echo "  Next:"
echo "    git checkout feature/syna30b8-initial-probe"
echo "    cargo run -- probe"
echo ""