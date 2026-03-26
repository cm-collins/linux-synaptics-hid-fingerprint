#!/usr/bin/env bash
# ============================================================
# post-create.sh — Runs once after container is first created
# ============================================================
set -euo pipefail

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo -e "${CYAN}"
echo "╔═══════════════════════════════════════════════════╗"
echo "║  🦀  SYNA30B8 Fingerprint Driver - Dev Setup      ║"
echo "║      HP EliteBook x360 1040 G7                    ║"
echo "║      Sensor: Synaptics 06CB:CE1A                  ║"
echo "╚═══════════════════════════════════════════════════╝"
echo -e "${NC}"

# ── Rust version info ─────────────────────────────────────
echo -e "${GREEN}✅ Rust toolchain:${NC}"
rustc --version
cargo --version
echo ""

# ── GitHub CLI auth status ────────────────────────────────
echo -e "${GREEN}✅ GitHub CLI:${NC}"
gh --version
echo ""

# ── Azure CLI version ─────────────────────────────────────
echo -e "${GREEN}✅ Azure CLI:${NC}"
az --version | head -1
echo ""

# ── Scaffold Cargo.toml if not present ───────────────────
if [ ! -f "Cargo.toml" ]; then
  echo -e "${YELLOW}📦 Initializing Cargo project...${NC}"
  cargo init --name syna30b8-fingerprint
fi

# ── Create src structure ──────────────────────────────────
mkdir -p src tests docs scripts

# ── Write Cargo.toml with all dependencies ───────────────
cat > Cargo.toml << 'CARGO'
[package]
name = "syna30b8-fingerprint"
version = "0.1.0"
edition = "2021"
authors = ["Munene"]
description = "Linux fingerprint driver for Synaptics SYNA30B8 (HP EliteBook x360 1040 G7)"
license = "GPL-2.0"
repository = "https://github.com/munene/syna30b8-fingerprint"
keywords = ["fingerprint", "hid", "linux", "driver", "synaptics"]
categories = ["hardware-support", "os::linux-apis"]

[[bin]]
name = "syna30b8"
path = "src/main.rs"

[lib]
name = "syna30b8_lib"
path = "src/lib.rs"

[dependencies]
# HID device access
hidapi = { version = "2.6", features = ["linux-native"] }

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
log = "0.4"
env_logger = "0.11"

# Async runtime (for future async HID reads)
tokio = { version = "1.0", features = ["full"] }

# Serialization (saving fingerprint templates)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Hex encoding/decoding (HID report display)
hex = "0.4"

# Byte manipulation (HID report parsing)
byteorder = "1.5"
bytes = "1.5"

# CLI argument parsing
clap = { version = "4.0", features = ["derive"] }

# Configuration
config = "0.14"
dirs = "5.0"

[dev-dependencies]
# Better test assertions
pretty_assertions = "1.4"
# Test fixtures
tempfile = "3.8"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1

[profile.dev]
opt-level = 0
debug = true
CARGO

echo -e "${GREEN}✅ Cargo.toml written${NC}"

# ── Write main.rs ─────────────────────────────────────────
cat > src/main.rs << 'RUST'
use anyhow::Result;
use clap::{Parser, Subcommand};
use syna30b8_lib::{sensor::Syna30b8Sensor, FingerprintSensor};

#[derive(Parser)]
#[command(name = "syna30b8")]
#[command(about = "Synaptics SYNA30B8 Fingerprint Driver - HP EliteBook x360 1040 G7")]
#[command(version = "0.1.0")]
#[command(author = "Munene")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Probe the sensor and display device info
    Probe,
    /// Listen for raw HID reports (place finger on sensor)
    Listen,
    /// Enroll a fingerprint
    Enroll,
    /// Verify a fingerprint
    Verify,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Probe => {
            let sensor = Syna30b8Sensor::open()?;
            sensor.probe()?;
        }
        Commands::Listen => {
            let sensor = Syna30b8Sensor::open()?;
            sensor.listen()?;
        }
        Commands::Enroll => {
            println!("🔜 Enroll - coming in Phase 3");
        }
        Commands::Verify => {
            println!("🔜 Verify - coming in Phase 4");
        }
    }

    Ok(())
}
RUST

# ── Write lib.rs ──────────────────────────────────────────
cat > src/lib.rs << 'RUST'
pub mod sensor;
pub mod protocol;
pub mod error;

pub use error::SensorError;

/// Core trait - every sensor (real or mock) implements this
pub trait FingerprintSensor {
    fn probe(&self) -> Result<(), SensorError>;
    fn listen(&self) -> Result<(), SensorError>;
    fn read_report(&self) -> Result<Vec<u8>, SensorError>;
}
RUST

# ── Write sensor.rs ───────────────────────────────────────
cat > src/sensor.rs << 'RUST'
use crate::{FingerprintSensor, SensorError};
use hidapi::HidApi;
use log::{debug, info};

/// Vendor and Product ID for SYNA30B8 on HP EliteBook x360 1040 G7
pub const SYNAPTICS_VID: u16 = 0x06CB;
pub const SYNA30B8_PID: u16 = 0xCE1A;

pub struct Syna30b8Sensor {
    device: hidapi::HidDevice,
}

impl Syna30b8Sensor {
    pub fn open() -> Result<Self, SensorError> {
        let api = HidApi::new().map_err(|e| SensorError::HidInit(e.to_string()))?;

        info!("Opening SYNA30B8 sensor (VID:{:04X} PID:{:04X})", SYNAPTICS_VID, SYNA30B8_PID);

        let device = api
            .open(SYNAPTICS_VID, SYNA30B8_PID)
            .map_err(|e| SensorError::DeviceNotFound(e.to_string()))?;

        Ok(Self { device })
    }
}

impl FingerprintSensor for Syna30b8Sensor {
    fn probe(&self) -> Result<(), SensorError> {
        println!("\n🔍 SYNA30B8 Sensor Info");
        println!("─────────────────────────────────────");
        println!("  VID          : 0x{:04X} (Synaptics)", SYNAPTICS_VID);
        println!("  PID          : 0x{:04X} (SYNA30B8)", SYNA30B8_PID);
        println!("  Manufacturer : {:?}", self.device.get_manufacturer_string());
        println!("  Product      : {:?}", self.device.get_product_string());
        println!("  Serial       : {:?}", self.device.get_serial_number_string());
        println!("─────────────────────────────────────");
        println!("✅ Sensor reachable!\n");
        Ok(())
    }

    fn listen(&self) -> Result<(), SensorError> {
        println!("\n📡 Listening for HID reports...");
        println!("   Place your finger on the sensor");
        println!("   Press Ctrl+C to stop\n");

        let mut buf = [0u8; 256];
        loop {
            match self.device.read_timeout(&mut buf, 5000) {
                Ok(0) => println!("⏱  Timeout - waiting..."),
                Ok(size) => {
                    debug!("Raw report: {}", hex::encode(&buf[..size]));
                    println!("📦 Report ({} bytes)", size);
                    println!("   Report ID : 0x{:02X}", buf[0]);
                    println!("   Hex       : {}", hex::encode(&buf[..size]));
                    println!("   Bytes     : {:?}\n", &buf[1..size]);
                }
                Err(e) => return Err(SensorError::ReadError(e.to_string())),
            }
        }
    }

    fn read_report(&self) -> Result<Vec<u8>, SensorError> {
        let mut buf = [0u8; 256];
        let size = self.device
            .read_timeout(&mut buf, 5000)
            .map_err(|e| SensorError::ReadError(e.to_string()))?;
        Ok(buf[..size].to_vec())
    }
}

/// Mock sensor for testing without hardware
pub struct MockSensor {
    pub responses: Vec<Vec<u8>>,
}

impl MockSensor {
    pub fn new() -> Self {
        Self {
            responses: vec![
                vec![0x03, 0x01, 0x00, 0xFF, 0x7F],
                vec![0x03, 0x02, 0x01, 0xAB, 0xCD],
            ],
        }
    }
}

impl FingerprintSensor for MockSensor {
    fn probe(&self) -> Result<(), SensorError> {
        println!("🧪 Mock sensor probed successfully");
        Ok(())
    }

    fn listen(&self) -> Result<(), SensorError> {
        for r in &self.responses {
            println!("🧪 Mock report: {}", hex::encode(r));
        }
        Ok(())
    }

    fn read_report(&self) -> Result<Vec<u8>, SensorError> {
        Ok(self.responses[0].clone())
    }
}
RUST

# ── Write protocol.rs ─────────────────────────────────────
cat > src/protocol.rs << 'RUST'
/// HID Report IDs observed from SYNA30B8 descriptor
/// These will be updated as we reverse-engineer the protocol
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum ReportId {
    MouseButtons     = 0x02,
    TouchContact     = 0x03,  // Currently mapped as multitouch
    FingerprintData  = 0x08,  // Hypothesis - needs verification
    VendorSpecific   = 0x09,
    Unknown(u8),
}

impl From<u8> for ReportId {
    fn from(id: u8) -> Self {
        match id {
            0x02 => ReportId::MouseButtons,
            0x03 => ReportId::TouchContact,
            0x08 => ReportId::FingerprintData,
            0x09 => ReportId::VendorSpecific,
            other => ReportId::Unknown(other),
        }
    }
}

/// A raw HID report from the sensor
#[derive(Debug)]
pub struct HidReport {
    pub id: ReportId,
    pub data: Vec<u8>,
}

impl HidReport {
    pub fn parse(raw: &[u8]) -> Option<Self> {
        if raw.is_empty() {
            return None;
        }
        Some(Self {
            id: ReportId::from(raw[0]),
            data: raw[1..].to_vec(),
        })
    }
}
RUST

# ── Write error.rs ────────────────────────────────────────
cat > src/error.rs << 'RUST'
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SensorError {
    #[error("Failed to initialize HID API: {0}")]
    HidInit(String),

    #[error("SYNA30B8 device not found. Is it plugged in? Try: sudo cargo run\nError: {0}")]
    DeviceNotFound(String),

    #[error("Failed to read from sensor: {0}")]
    ReadError(String),

    #[error("Failed to parse HID report: {0}")]
    ParseError(String),

    #[error("Protocol error: {0}")]
    ProtocolError(String),
}
RUST

# ── Write a test ──────────────────────────────────────────
cat > tests/mock_sensor_test.rs << 'RUST'
use syna30b8_lib::{sensor::MockSensor, FingerprintSensor};

#[test]
fn test_mock_sensor_probe() {
    let sensor = MockSensor::new();
    assert!(sensor.probe().is_ok());
}

#[test]
fn test_mock_sensor_read_report() {
    let sensor = MockSensor::new();
    let report = sensor.read_report().unwrap();
    assert!(!report.is_empty());
    assert_eq!(report[0], 0x03); // Expected first report ID
}
RUST

echo -e "${GREEN}✅ Project scaffolded${NC}"

# ── Initial cargo build check ─────────────────────────────
echo -e "\n${YELLOW}🔨 Running initial cargo check...${NC}"
cargo check && echo -e "${GREEN}✅ Cargo check passed!${NC}" || echo -e "${RED}⚠️  Fix errors above before proceeding${NC}"

echo -e "\n${GREEN}🎉 Dev environment ready!${NC}"
echo ""
echo "  Quick commands:"
echo "    cargo run -- probe      → Test sensor connection"
echo "    cargo run -- listen     → Read raw HID reports"
echo "    cargo nextest run       → Run all tests"
echo "    cargo watch -x check    → Auto-check on file save"
echo ""