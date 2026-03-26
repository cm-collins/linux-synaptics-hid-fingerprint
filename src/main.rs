use rusb::{
    ConfigDescriptor, Context, Device, DeviceDescriptor, Direction, TransferType, UsbContext,
};
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

const DEFAULT_VENDOR_ID: u16 = 0x06cb;
const DEFAULT_PRODUCT_ID: u16 = 0x00e9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecutionEnvironment {
    DevContainer,
    LocalMachine,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse(env::args().skip(1).collect())?;

    match args.command {
        Command::Probe(options) => probe(options),
        Command::DeviceProfile(options) => device_profile(options),
        Command::List => print_help(),
    }
}

fn probe(options: ProbeOptions) -> Result<(), String> {
    let matches = find_matching_devices(options.vendor_id, options.product_id)?;
    let mut report = String::new();
    append_line(
        &mut report,
        format!(
            "found {} matching device(s) for {:04x}:{:04x}",
            matches.len(),
            options.vendor_id,
            options.product_id
        ),
    );

    for (index, (device, descriptor)) in matches.into_iter().enumerate() {
        append_blank_line(&mut report);
        append_line(&mut report, format!("device #{}", index + 1));
        print_device_summary(&mut report, &device, &descriptor)?;
        maybe_probe_runtime(&mut report, &device, &options)?;
    }

    print!("{report}");

    if let Some(path) = options.output_path.as_deref() {
        fs::write(path, &report)
            .map_err(|err| format!("failed to write report to {path}: {err}"))?;
    }

    Ok(())
}

fn device_profile(options: ProbeOptions) -> Result<(), String> {
    let matches = find_matching_devices(options.vendor_id, options.product_id)?;
    let markdown = render_device_profile(&matches, &options)?;

    let output_path = options
        .output_path
        .clone()
        .unwrap_or_else(|| String::from("notes/device-profile.md"));

    fs::write(&output_path, &markdown)
        .map_err(|err| format!("failed to write profile to {output_path}: {err}"))?;

    print!("{markdown}");
    Ok(())
}

fn find_matching_devices(
    vendor_id: u16,
    product_id: u16,
) -> Result<Vec<(Device<Context>, DeviceDescriptor)>, String> {
    let context =
        Context::new().map_err(|err| format!("failed to create libusb context: {err}"))?;
    let devices = context
        .devices()
        .map_err(|err| format!("failed to enumerate USB devices: {err}"))?;

    let mut matches = Vec::new();
    for device in devices.iter() {
        let descriptor = device
            .device_descriptor()
            .map_err(|err| format!("failed to read device descriptor: {err}"))?;

        if descriptor.vendor_id() == vendor_id && descriptor.product_id() == product_id {
            matches.push((device, descriptor));
        }
    }

    if matches.is_empty() {
        return Err(render_device_not_found_message(
            vendor_id,
            product_id,
            &devices,
            detect_execution_environment(),
        ));
    }

    Ok(matches)
}

fn render_device_not_found_message(
    vendor_id: u16,
    product_id: u16,
    devices: &rusb::DeviceList<Context>,
    environment: ExecutionEnvironment,
) -> String {
    let visible_devices = devices
        .iter()
        .filter_map(|device| {
            let descriptor = device.device_descriptor().ok()?;
            Some(format!(
                "{:04x}:{:04x} on bus {:03} address {:03}",
                descriptor.vendor_id(),
                descriptor.product_id(),
                device.bus_number(),
                device.address()
            ))
        })
        .collect::<Vec<_>>();

    render_device_not_found_message_from_visible_devices(
        vendor_id,
        product_id,
        &visible_devices,
        environment,
    )
}

fn render_device_not_found_message_from_visible_devices(
    vendor_id: u16,
    product_id: u16,
    visible_devices: &[String],
    environment: ExecutionEnvironment,
) -> String {
    let mut message = format!(
        "device {:04x}:{:04x} not found on the USB bus",
        vendor_id, product_id
    );

    if visible_devices.is_empty() {
        message.push_str("; libusb did not enumerate any USB devices");
    } else {
        message.push_str(&format!(
            "; libusb currently sees {} device(s): {}",
            visible_devices.len(),
            visible_devices.join(", ")
        ));
    }

    match environment {
        ExecutionEnvironment::DevContainer => {
            message.push_str(
                ". Detected a containerized environment. Verify the fingerprint reader is attached on the host and that `/dev/bus/usb` is being passed through correctly.",
            );
        }
        ExecutionEnvironment::LocalMachine => {
            message.push_str(&format!(
                ". Detected a local machine environment. Verify the reader is attached and check `lsusb -d {:04x}:{:04x}`.",
                vendor_id, product_id
            ));
        }
    }

    message
}

fn detect_execution_environment() -> ExecutionEnvironment {
    let devcontainer_markers = [
        "REMOTE_CONTAINERS",
        "DEVCONTAINER",
        "CODESPACES",
        "container",
    ];

    if devcontainer_markers
        .iter()
        .any(|key| env::var_os(key).is_some())
        || Path::new("/.dockerenv").exists()
        || Path::new("/run/.containerenv").exists()
    {
        return ExecutionEnvironment::DevContainer;
    }

    ExecutionEnvironment::LocalMachine
}

fn print_device_summary<T: UsbContext>(
    report: &mut String,
    device: &Device<T>,
    descriptor: &DeviceDescriptor,
) -> Result<(), String> {
    let bus = device.bus_number();
    let address = device.address();

    append_line(
        report,
        format!("  location: bus {:03} address {:03}", bus, address),
    );
    append_line(
        report,
        format!(
            "  vendor/product: {:04x}:{:04x}",
            descriptor.vendor_id(),
            descriptor.product_id()
        ),
    );
    append_line(
        report,
        format!(
            "  class/subclass/protocol: {:02x}/{:02x}/{:02x}",
            descriptor.class_code(),
            descriptor.sub_class_code(),
            descriptor.protocol_code()
        ),
    );
    append_line(
        report,
        format!("  usb version: {}", descriptor.usb_version()),
    );
    append_line(
        report,
        format!("  max packet size (ep0): {}", descriptor.max_packet_size()),
    );
    append_line(
        report,
        format!("  configurations: {}", descriptor.num_configurations()),
    );

    for config_index in 0..descriptor.num_configurations() {
        let config = device
            .config_descriptor(config_index)
            .map_err(|err| format!("failed to read config {config_index}: {err}"))?;
        print_config_summary(report, &config);
    }

    Ok(())
}

fn print_config_summary(report: &mut String, config: &ConfigDescriptor) {
    append_blank_line(report);
    append_line(
        report,
        format!(
            "  config {}: interfaces={} self_powered={} remote_wakeup={} max_power={}mA",
            config.number(),
            config.num_interfaces(),
            config.self_powered(),
            config.remote_wakeup(),
            config.max_power()
        ),
    );

    for interface in config.interfaces() {
        append_line(report, format!("    interface {}:", interface.number()));
        for interface_desc in interface.descriptors() {
            append_line(
                report,
                format!(
                    "      alt {} class/subclass/protocol {:02x}/{:02x}/{:02x}",
                    interface_desc.setting_number(),
                    interface_desc.class_code(),
                    interface_desc.sub_class_code(),
                    interface_desc.protocol_code()
                ),
            );
            append_line(
                report,
                format!("      endpoints: {}", interface_desc.num_endpoints()),
            );

            for endpoint in interface_desc.endpoint_descriptors() {
                append_line(
                    report,
                    format!(
                        "        ep 0x{:02x} {} {} max_packet={} interval={}",
                        endpoint.address(),
                        direction_name(endpoint.direction()),
                        transfer_type_name(endpoint.transfer_type()),
                        endpoint.max_packet_size(),
                        endpoint.interval()
                    ),
                );
            }
        }
    }
}

fn maybe_probe_runtime<T: UsbContext>(
    report: &mut String,
    device: &Device<T>,
    options: &ProbeOptions,
) -> Result<(), String> {
    let Some(plan) = build_runtime_probe_plan(options)? else {
        return Ok(());
    };

    append_blank_line(report);
    append_line(
        report,
        format!("  runtime probe: claim interface {}", plan.interface),
    );

    let handle = device
        .open()
        .map_err(|err| format!("failed to open device for runtime probe: {err}"))?;

    match try_runtime_probe(report, handle, &plan) {
        Ok(()) => Ok(()),
        Err(err) => {
            append_line(report, format!("    runtime probe error: {err}"));
            Ok(())
        }
    }
}

fn try_runtime_probe<T: UsbContext>(
    report: &mut String,
    handle: rusb::DeviceHandle<T>,
    plan: &RuntimeProbePlan,
) -> Result<(), String> {
    match handle.kernel_driver_active(plan.interface) {
        Ok(true) => append_line(report, "    kernel driver active: yes".to_string()),
        Ok(false) => append_line(report, "    kernel driver active: no".to_string()),
        Err(err) => append_line(
            report,
            format!("    kernel driver active: unavailable ({err})"),
        ),
    }

    handle
        .claim_interface(plan.interface)
        .map_err(|err| format!("failed to claim interface {}: {err}", plan.interface))?;
    append_line(
        report,
        format!("    claim result: ok (interface {})", plan.interface),
    );

    if let Some(read_request) = plan.read_request {
        let timeout = Duration::from_millis(read_request.timeout_ms);
        let mut buffer = vec![0u8; usize::from(read_request.length)];
        append_line(
            report,
            format!(
                "    bounded read: endpoint=0x{endpoint:02x} length={} timeout_ms={}",
                read_request.length,
                read_request.timeout_ms,
                endpoint = read_request.endpoint
            ),
        );

        let read_result = handle
            .read_interrupt(read_request.endpoint, &mut buffer, timeout)
            .or_else(|interrupt_err| {
                handle
                    .read_bulk(read_request.endpoint, &mut buffer, timeout)
                    .map_err(|bulk_err| {
                        format!(
                            "interrupt read error: {interrupt_err}; bulk read error: {bulk_err}"
                        )
                    })
            });

        match read_result {
            Ok(bytes_read) => {
                append_line(report, format!("    read result: {} byte(s)", bytes_read));
                append_line(
                    report,
                    format!(
                        "    read bytes: {}",
                        format_hex_bytes(&buffer[..bytes_read])
                    ),
                );
            }
            Err(err) => {
                append_line(report, format!("    read result: {err}"));
            }
        }
    }

    handle
        .release_interface(plan.interface)
        .map_err(|err| format!("failed to release interface {}: {err}", plan.interface))?;
    append_line(
        report,
        format!("    release result: ok (interface {})", plan.interface),
    );

    Ok(())
}

fn build_runtime_probe_plan(options: &ProbeOptions) -> Result<Option<RuntimeProbePlan>, String> {
    let Some(interface) = options.claim_interface else {
        return Ok(None);
    };

    let read_request = options
        .read_endpoint
        .map(|endpoint| {
            build_bounded_read_request(endpoint, options.read_length, options.timeout_ms)
        })
        .transpose()?;

    Ok(Some(RuntimeProbePlan {
        interface,
        read_request,
    }))
}

fn build_bounded_read_request(
    endpoint: u8,
    length: u16,
    timeout_ms: u64,
) -> Result<BoundedReadRequest, String> {
    if !is_in_endpoint(endpoint) {
        return Err(format!(
            "endpoint 0x{endpoint:02x} is not an IN endpoint; bounded reads require an IN endpoint"
        ));
    }

    Ok(BoundedReadRequest {
        endpoint,
        length,
        timeout_ms,
    })
}

fn is_in_endpoint(endpoint: u8) -> bool {
    endpoint & 0x80 != 0
}

fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::In => "IN",
        Direction::Out => "OUT",
    }
}

fn transfer_type_name(transfer_type: TransferType) -> &'static str {
    match transfer_type {
        TransferType::Control => "control",
        TransferType::Isochronous => "isochronous",
        TransferType::Bulk => "bulk",
        TransferType::Interrupt => "interrupt",
    }
}

fn print_help() -> Result<(), String> {
    println!("syna-tool");
    println!();
    println!("Commands:");
    println!("  probe                  Inspect the target Synaptics USB device");
    println!("  probe --vid 06cb --pid 00e9");
    println!("  probe --output artifacts/probe.txt");
    println!("  probe --claim 0 --read-ep 0x83 --length 64 --timeout-ms 250");
    println!("  device-profile         Write a Markdown device profile");
    println!("  device-profile --output notes/device-profile.md");
    println!("  help                   Show this help");
    Ok(())
}

#[derive(Debug)]
struct Args {
    command: Command,
}

#[derive(Debug)]
enum Command {
    Probe(ProbeOptions),
    DeviceProfile(ProbeOptions),
    List,
}

#[derive(Debug, PartialEq, Eq)]
struct ProbeOptions {
    vendor_id: u16,
    product_id: u16,
    output_path: Option<String>,
    claim_interface: Option<u8>,
    read_endpoint: Option<u8>,
    read_length: u16,
    timeout_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeProbePlan {
    interface: u8,
    read_request: Option<BoundedReadRequest>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BoundedReadRequest {
    endpoint: u8,
    length: u16,
    timeout_ms: u64,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            command: Command::Probe(ProbeOptions::default()),
        }
    }
}

impl Default for ProbeOptions {
    fn default() -> Self {
        Self {
            vendor_id: DEFAULT_VENDOR_ID,
            product_id: DEFAULT_PRODUCT_ID,
            output_path: None,
            claim_interface: None,
            read_endpoint: None,
            read_length: 64,
            timeout_ms: 250,
        }
    }
}

impl Args {
    fn parse(tokens: Vec<String>) -> Result<Self, String> {
        if tokens.is_empty() {
            return Ok(Self::default());
        }

        let command = tokens[0].as_str();
        match command {
            "probe" => parse_probe_args(&tokens[1..]),
            "device-profile" => parse_device_profile_args(&tokens[1..]),
            "help" | "--help" | "-h" => Ok(Self {
                command: Command::List,
            }),
            other => Err(format!("unknown command: {other}")),
        }
    }
}

fn parse_probe_args(tokens: &[String]) -> Result<Args, String> {
    let mut options = ProbeOptions::default();

    let mut index = 0;
    while index < tokens.len() {
        match tokens[index].as_str() {
            "--vid" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --vid"))?;
                options.vendor_id = parse_hex_u16(value)?;
                index += 2;
            }
            "--pid" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --pid"))?;
                options.product_id = parse_hex_u16(value)?;
                index += 2;
            }
            "--output" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --output"))?;
                options.output_path = Some(value.clone());
                index += 2;
            }
            "--claim" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --claim"))?;
                options.claim_interface = Some(parse_u8(value, "--claim")?);
                index += 2;
            }
            "--read-ep" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --read-ep"))?;
                options.read_endpoint = Some(parse_hex_u8(value)?);
                index += 2;
            }
            "--length" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --length"))?;
                options.read_length = parse_u16(value, "--length")?;
                index += 2;
            }
            "--timeout-ms" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --timeout-ms"))?;
                options.timeout_ms = parse_u64(value, "--timeout-ms")?;
                index += 2;
            }
            "--help" | "-h" => {
                return Ok(Args {
                    command: Command::List,
                });
            }
            other => return Err(format!("unknown probe option: {other}")),
        }
    }

    if options.read_endpoint.is_some() && options.claim_interface.is_none() {
        return Err(String::from(
            "--read-ep requires --claim so the tool can safely claim the interface first",
        ));
    }

    Ok(Args {
        command: Command::Probe(options),
    })
}

fn parse_device_profile_args(tokens: &[String]) -> Result<Args, String> {
    let Args { command } = parse_probe_args(tokens)?;
    let Command::Probe(mut options) = command else {
        return Err(String::from(
            "device-profile argument parsing unexpectedly produced a non-probe command",
        ));
    };

    if options.output_path.is_none() {
        options.output_path = Some(String::from("notes/device-profile.md"));
    }

    Ok(Args {
        command: Command::DeviceProfile(options),
    })
}

fn parse_hex_u16(value: &str) -> Result<u16, String> {
    let trimmed = value.trim_start_matches("0x");
    u16::from_str_radix(trimmed, 16).map_err(|_| format!("invalid hex value: {value}"))
}

fn parse_hex_u8(value: &str) -> Result<u8, String> {
    let trimmed = value.trim_start_matches("0x");
    u8::from_str_radix(trimmed, 16).map_err(|_| format!("invalid hex value: {value}"))
}

fn parse_u8(value: &str, flag_name: &str) -> Result<u8, String> {
    value
        .parse::<u8>()
        .map_err(|_| format!("invalid value for {flag_name}: {value}"))
}

fn parse_u16(value: &str, flag_name: &str) -> Result<u16, String> {
    value
        .parse::<u16>()
        .map_err(|_| format!("invalid value for {flag_name}: {value}"))
}

fn parse_u64(value: &str, flag_name: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid value for {flag_name}: {value}"))
}

fn format_hex_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::from("<empty>");
    }

    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn append_line(report: &mut String, line: String) {
    report.push_str(&line);
    report.push('\n');
}

fn append_blank_line(report: &mut String) {
    report.push('\n');
}

fn render_device_profile<T: UsbContext>(
    matches: &[(Device<T>, DeviceDescriptor)],
    options: &ProbeOptions,
) -> Result<String, String> {
    let mut markdown = String::new();
    append_line(&mut markdown, String::from("# Device Profile"));
    append_blank_line(&mut markdown);
    append_line(
        &mut markdown,
        format!(
            "Target USB ID: `{:04x}:{:04x}`",
            options.vendor_id, options.product_id
        ),
    );
    append_blank_line(&mut markdown);
    append_line(&mut markdown, String::from("## Capture Notes"));
    append_blank_line(&mut markdown);
    append_line(
        &mut markdown,
        String::from("- Generated by `cargo run -- device-profile`."),
    );
    if options.claim_interface.is_some() {
        append_line(
            &mut markdown,
            String::from("- This run included a bounded runtime probe request."),
        );
    } else {
        append_line(
            &mut markdown,
            String::from("- This run is descriptor-only and did not claim the interface."),
        );
    }
    append_line(
        &mut markdown,
        format!("- Matching devices seen during this run: {}", matches.len()),
    );
    append_blank_line(&mut markdown);
    append_line(&mut markdown, String::from("## Confirmed During This Run"));
    append_blank_line(&mut markdown);
    append_line(
        &mut markdown,
        format!(
            "- USB device enumerated as `{:04x}:{:04x}`.",
            options.vendor_id, options.product_id
        ),
    );
    append_line(
        &mut markdown,
        String::from("- Descriptor and endpoint details were collected from the live USB bus."),
    );
    append_blank_line(&mut markdown);
    append_line(&mut markdown, String::from("## Outstanding Questions"));
    append_blank_line(&mut markdown);
    append_line(
        &mut markdown,
        String::from(
            "- Confirm which interface and alternate setting should be used for safe probing.",
        ),
    );
    append_line(
        &mut markdown,
        String::from("- Determine whether interrupt traffic appears while the device is idle."),
    );
    append_line(
        &mut markdown,
        String::from("- Determine whether startup behavior changes after interface claim."),
    );

    for (index, (device, descriptor)) in matches.iter().enumerate() {
        append_blank_line(&mut markdown);
        append_line(&mut markdown, format!("## Device {}", index + 1));
        append_blank_line(&mut markdown);
        append_line(
            &mut markdown,
            format!(
                "- Bus/address: `{:03}/{:03}`",
                device.bus_number(),
                device.address()
            ),
        );
        append_line(
            &mut markdown,
            format!("- USB version: `{}`", descriptor.usb_version()),
        );
        append_line(
            &mut markdown,
            format!(
                "- Class/subclass/protocol: `{:02x}/{:02x}/{:02x}`",
                descriptor.class_code(),
                descriptor.sub_class_code(),
                descriptor.protocol_code()
            ),
        );
        append_line(
            &mut markdown,
            format!("- EP0 max packet size: `{}`", descriptor.max_packet_size()),
        );
        append_line(
            &mut markdown,
            format!("- Configurations: `{}`", descriptor.num_configurations()),
        );

        for config_index in 0..descriptor.num_configurations() {
            let config = device
                .config_descriptor(config_index)
                .map_err(|err| format!("failed to read config {config_index}: {err}"))?;
            append_blank_line(&mut markdown);
            append_line(&mut markdown, format!("### Config {}", config.number()));
            append_blank_line(&mut markdown);
            append_line(
                &mut markdown,
                format!(
                    "- Interfaces: `{}`; self powered: `{}`; remote wakeup: `{}`; max power: `{}mA`",
                    config.num_interfaces(),
                    config.self_powered(),
                    config.remote_wakeup(),
                    config.max_power()
                ),
            );

            for interface in config.interfaces() {
                append_line(
                    &mut markdown,
                    format!("- Interface `{}`", interface.number()),
                );
                for interface_desc in interface.descriptors() {
                    append_line(
                        &mut markdown,
                        format!(
                            "  Alt `{}` class/subclass/protocol `{:02x}/{:02x}/{:02x}` endpoints `{}`",
                            interface_desc.setting_number(),
                            interface_desc.class_code(),
                            interface_desc.sub_class_code(),
                            interface_desc.protocol_code(),
                            interface_desc.num_endpoints()
                        ),
                    );
                    for endpoint in interface_desc.endpoint_descriptors() {
                        append_line(
                            &mut markdown,
                            format!(
                                "  Endpoint `0x{:02x}` {} {} max packet `{}` interval `{}`",
                                endpoint.address(),
                                direction_name(endpoint.direction()),
                                transfer_type_name(endpoint.transfer_type()),
                                endpoint.max_packet_size(),
                                endpoint.interval()
                            ),
                        );
                    }
                }
            }
        }

        let runtime_summary = render_runtime_profile_section(device, options)?;
        append_blank_line(&mut markdown);
        append_line(&mut markdown, String::from("### Runtime Probe"));
        append_blank_line(&mut markdown);
        for line in runtime_summary.lines() {
            append_line(&mut markdown, format!("- {line}"));
        }
    }

    Ok(markdown)
}

fn render_runtime_profile_section<T: UsbContext>(
    device: &Device<T>,
    options: &ProbeOptions,
) -> Result<String, String> {
    let mut report = String::new();
    if options.claim_interface.is_none() {
        append_line(
            &mut report,
            String::from("No runtime probe requested for this profile run."),
        );
        return Ok(report.trim_end().to_string());
    }

    maybe_probe_runtime(&mut report, device, options)?;
    Ok(report
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n"))
}

#[cfg(test)]
mod tests {
    use super::{
        Args, BoundedReadRequest, Command, ExecutionEnvironment, ProbeOptions, RuntimeProbePlan,
        build_runtime_probe_plan, parse_hex_u16,
        render_device_not_found_message_from_visible_devices,
    };

    #[test]
    fn parses_default_probe_command() {
        let args = Args::parse(Vec::new()).expect("default args should parse");
        match args.command {
            Command::Probe(options) => assert_eq!(options, ProbeOptions::default()),
            Command::DeviceProfile(_) => panic!("expected probe command"),
            Command::List => panic!("expected probe command"),
        }
    }

    #[test]
    fn parses_probe_overrides() {
        let tokens = vec![
            String::from("probe"),
            String::from("--vid"),
            String::from("1234"),
            String::from("--pid"),
            String::from("abcd"),
        ];
        let args = Args::parse(tokens).expect("probe args should parse");
        match args.command {
            Command::Probe(options) => {
                assert_eq!(options.vendor_id, 0x1234);
                assert_eq!(options.product_id, 0xabcd);
            }
            Command::DeviceProfile(_) => panic!("expected probe command"),
            Command::List => panic!("expected probe command"),
        }
    }

    #[test]
    fn accepts_prefixed_hex_values() {
        let value = parse_hex_u16("0x00e9").expect("hex value should parse");
        assert_eq!(value, 0x00e9);
    }

    #[test]
    fn parses_artifact_and_runtime_probe_options() {
        let tokens = vec![
            String::from("probe"),
            String::from("--output"),
            String::from("artifacts/probe.txt"),
            String::from("--claim"),
            String::from("0"),
            String::from("--read-ep"),
            String::from("0x83"),
            String::from("--length"),
            String::from("32"),
            String::from("--timeout-ms"),
            String::from("500"),
        ];
        let args = Args::parse(tokens).expect("probe args should parse");
        match args.command {
            Command::Probe(options) => {
                assert_eq!(options.output_path.as_deref(), Some("artifacts/probe.txt"));
                assert_eq!(options.claim_interface, Some(0));
                assert_eq!(options.read_endpoint, Some(0x83));
                assert_eq!(options.read_length, 32);
                assert_eq!(options.timeout_ms, 500);
            }
            Command::DeviceProfile(_) => panic!("expected probe command"),
            Command::List => panic!("expected probe command"),
        }
    }

    #[test]
    fn rejects_read_without_claim() {
        let tokens = vec![
            String::from("probe"),
            String::from("--read-ep"),
            String::from("0x83"),
        ];
        let error = Args::parse(tokens).expect_err("read without claim should fail");
        assert!(error.contains("--read-ep requires --claim"));
    }

    #[test]
    fn parses_device_profile_command() {
        let tokens = vec![String::from("device-profile")];
        let args = Args::parse(tokens).expect("device-profile args should parse");
        match args.command {
            Command::DeviceProfile(options) => {
                assert_eq!(
                    options.output_path.as_deref(),
                    Some("notes/device-profile.md")
                );
                assert_eq!(options.vendor_id, 0x06cb);
                assert_eq!(options.product_id, 0x00e9);
            }
            Command::Probe(_) => panic!("expected device-profile command"),
            Command::List => panic!("expected device-profile command"),
        }
    }

    #[test]
    fn not_found_message_includes_visible_devices_and_hint() {
        let visible_devices = vec![
            String::from("06cb:00e9 on bus 001 address 003"),
            String::from("1d6b:0003 on bus 002 address 001"),
        ];

        let message = render_device_not_found_message_from_visible_devices(
            0x06cb,
            0x00e9,
            &visible_devices,
            ExecutionEnvironment::DevContainer,
        );

        assert!(message.contains("device 06cb:00e9 not found on the USB bus"));
        assert!(message.contains("libusb currently sees"));
        assert!(message.contains("/dev/bus/usb"));
    }

    #[test]
    fn not_found_message_supports_local_machine_guidance() {
        let message = render_device_not_found_message_from_visible_devices(
            0x06cb,
            0x00e9,
            &[],
            ExecutionEnvironment::LocalMachine,
        );

        assert!(message.contains("libusb did not enumerate any USB devices"));
        assert!(message.contains("Detected a local machine environment"));
        assert!(message.contains("lsusb -d 06cb:00e9"));
    }

    #[test]
    fn runtime_probe_plan_is_empty_without_claim() {
        let plan = build_runtime_probe_plan(&ProbeOptions::default())
            .expect("default runtime probe plan should build");

        assert_eq!(plan, None);
    }

    #[test]
    fn runtime_probe_plan_includes_bounded_read_for_in_endpoint() {
        let options = ProbeOptions {
            claim_interface: Some(0),
            read_endpoint: Some(0x83),
            read_length: 32,
            timeout_ms: 500,
            ..ProbeOptions::default()
        };

        let plan = build_runtime_probe_plan(&options).expect("runtime probe plan should build");

        assert_eq!(
            plan,
            Some(RuntimeProbePlan {
                interface: 0,
                read_request: Some(BoundedReadRequest {
                    endpoint: 0x83,
                    length: 32,
                    timeout_ms: 500,
                }),
            })
        );
    }

    #[test]
    fn runtime_probe_plan_rejects_out_endpoint_reads() {
        let options = ProbeOptions {
            claim_interface: Some(0),
            read_endpoint: Some(0x01),
            ..ProbeOptions::default()
        };

        let error =
            build_runtime_probe_plan(&options).expect_err("OUT endpoint reads should be rejected");

        assert!(error.contains("endpoint 0x01 is not an IN endpoint"));
    }
}
