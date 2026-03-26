use rusb::{
    ConfigDescriptor, Context, Device, DeviceDescriptor, Direction, TransferType, UsbContext,
};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_VENDOR_ID: u16 = 0x06cb;
const DEFAULT_PRODUCT_ID: u16 = 0x00e9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExecutionEnvironment {
    DevContainer,
    LocalMachine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CaptureDirection {
    In,
    Out,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CaptureTransferType {
    Bulk,
    Interrupt,
    Control,
    Isochronous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UsbmonEventKind {
    Submit,
    Complete,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadTransport {
    Interrupt,
    Bulk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeReadStrategy {
    Auto,
    Interrupt,
    Bulk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct UsbControlSetup {
    request_type: u8,
    request: u8,
    value: u16,
    index: u16,
    length: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RuntimeTraceEntry {
    attempt: u16,
    timestamp_ms: u128,
    elapsed_ms: u128,
    transport: Option<ReadTransport>,
    result: RuntimeTraceResult,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum RuntimeTraceResult {
    Success(Vec<u8>),
    Failure(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UsbmonRecord {
    request_tag: String,
    timestamp_us: u64,
    event_kind: UsbmonEventKind,
    transfer_type: CaptureTransferType,
    direction: CaptureDirection,
    bus_number: u16,
    device_address: u16,
    endpoint: u8,
    status: i32,
    length: u32,
    payload: Vec<u8>,
    control_setup: Option<UsbControlSetup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UsbmonAnalysisSummary {
    total_records: usize,
    endpoint_summaries: Vec<EndpointSummary>,
    control_request_summaries: Vec<ControlRequestSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct EndpointSummary {
    transfer_type: CaptureTransferType,
    direction: CaptureDirection,
    bus_number: u16,
    device_address: u16,
    endpoint: u8,
    total_records: usize,
    submit_count: usize,
    complete_count: usize,
    error_count: usize,
    length_counts: BTreeMap<u32, usize>,
    status_counts: BTreeMap<i32, usize>,
    submit_timing: Option<TimingSummary>,
    completion_latency: Option<TimingSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ControlRequestSummary {
    setup: UsbControlSetup,
    count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimingSummary {
    sample_count: usize,
    min_us: u64,
    max_us: u64,
    avg_us: u64,
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
        Command::RuntimeTrace(options) => runtime_trace(options),
        Command::AnalyzeUsbmon(options) => analyze_usbmon(options),
        Command::CompareUsbmon(options) => compare_usbmon(options),
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

fn runtime_trace(options: RuntimeTraceOptions) -> Result<(), String> {
    let matches = find_matching_devices(options.vendor_id, options.product_id)?;
    let trace_plan = build_runtime_trace_plan(&options)?;
    let mut report = String::new();

    append_line(
        &mut report,
        format!(
            "runtime trace for {:04x}:{:04x} with {} matching device(s)",
            options.vendor_id,
            options.product_id,
            matches.len()
        ),
    );

    for (index, (device, descriptor)) in matches.into_iter().enumerate() {
        append_blank_line(&mut report);
        append_line(&mut report, format!("device #{}", index + 1));
        print_device_summary(&mut report, &device, &descriptor)?;
        run_runtime_trace_for_device(&mut report, &device, &trace_plan)?;
    }

    print!("{report}");

    if let Some(path) = options.output_path.as_deref() {
        fs::write(path, &report)
            .map_err(|err| format!("failed to write runtime trace to {path}: {err}"))?;
    }

    Ok(())
}

fn analyze_usbmon(options: UsbmonAnalysisOptions) -> Result<(), String> {
    let input = fs::read_to_string(&options.input_path).map_err(|err| {
        format!(
            "failed to read usbmon capture {}: {err}",
            options.input_path
        )
    })?;
    let records = parse_usbmon_capture(&input)?;
    let filtered = filter_usbmon_records(&records, options.bus_number, options.device_address);
    let summary = summarize_usbmon_records(&filtered);
    let report = render_usbmon_analysis(&summary, &options);

    print!("{report}");

    if let Some(path) = options.output_path.as_deref() {
        fs::write(path, &report)
            .map_err(|err| format!("failed to write usbmon analysis to {path}: {err}"))?;
    }

    Ok(())
}

fn compare_usbmon(options: UsbmonCompareOptions) -> Result<(), String> {
    let left_input = fs::read_to_string(&options.left_input_path).map_err(|err| {
        format!(
            "failed to read left usbmon capture {}: {err}",
            options.left_input_path
        )
    })?;
    let right_input = fs::read_to_string(&options.right_input_path).map_err(|err| {
        format!(
            "failed to read right usbmon capture {}: {err}",
            options.right_input_path
        )
    })?;

    let left_records = parse_usbmon_capture(&left_input)?;
    let right_records = parse_usbmon_capture(&right_input)?;
    let left_filtered =
        filter_usbmon_records(&left_records, options.bus_number, options.device_address);
    let right_filtered =
        filter_usbmon_records(&right_records, options.bus_number, options.device_address);
    let left_summary = summarize_usbmon_records(&left_filtered);
    let right_summary = summarize_usbmon_records(&right_filtered);
    let report = render_usbmon_comparison(&left_summary, &right_summary, &options);

    print!("{report}");

    if let Some(path) = options.output_path.as_deref() {
        fs::write(path, &report)
            .map_err(|err| format!("failed to write usbmon comparison to {path}: {err}"))?;
    }

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
            build_bounded_read_request(
                endpoint,
                options.read_length,
                options.timeout_ms,
                RuntimeReadStrategy::Auto,
            )
        })
        .transpose()?;

    Ok(Some(RuntimeProbePlan {
        interface,
        read_request,
    }))
}

fn build_runtime_trace_plan(options: &RuntimeTraceOptions) -> Result<RuntimeTracePlan, String> {
    Ok(RuntimeTracePlan {
        interface: options.claim_interface,
        read_request: build_bounded_read_request(
            options.read_endpoint,
            options.read_length,
            options.timeout_ms,
            options.read_strategy,
        )?,
        iterations: options.iterations,
        delay_ms: options.delay_ms,
    })
}

fn build_bounded_read_request(
    endpoint: u8,
    length: u16,
    timeout_ms: u64,
    strategy: RuntimeReadStrategy,
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
        strategy,
    })
}

fn is_in_endpoint(endpoint: u8) -> bool {
    endpoint & 0x80 != 0
}

fn run_runtime_trace_for_device<T: UsbContext>(
    report: &mut String,
    device: &Device<T>,
    plan: &RuntimeTracePlan,
) -> Result<(), String> {
    append_blank_line(report);
    append_line(
        report,
        format!(
            "  runtime trace: claim interface {} and read endpoint 0x{:02x} via {}",
            plan.interface,
            plan.read_request.endpoint,
            runtime_read_strategy_name(plan.read_request.strategy)
        ),
    );

    let handle = device
        .open()
        .map_err(|err| format!("failed to open device for runtime trace: {err}"))?;

    match execute_runtime_trace(report, handle, plan) {
        Ok(()) => Ok(()),
        Err(err) => {
            append_line(report, format!("    runtime trace error: {err}"));
            Ok(())
        }
    }
}

fn execute_runtime_trace<T: UsbContext>(
    report: &mut String,
    handle: rusb::DeviceHandle<T>,
    plan: &RuntimeTracePlan,
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

    let started = Instant::now();
    for attempt in 0..plan.iterations {
        let entry = capture_runtime_trace_entry(&handle, plan, attempt + 1, started);
        append_runtime_trace_entry(report, &entry);

        if plan.delay_ms > 0 && attempt + 1 < plan.iterations {
            thread::sleep(Duration::from_millis(plan.delay_ms));
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

fn capture_runtime_trace_entry<T: UsbContext>(
    handle: &rusb::DeviceHandle<T>,
    plan: &RuntimeTracePlan,
    attempt: u16,
    started: Instant,
) -> RuntimeTraceEntry {
    let timestamp_ms = unix_timestamp_ms();
    let elapsed_ms = started.elapsed().as_millis();
    let timeout = Duration::from_millis(plan.read_request.timeout_ms);
    let mut buffer = vec![0u8; usize::from(plan.read_request.length)];
    let (transport, result) = read_runtime_trace_payload(
        handle,
        &mut buffer,
        plan.read_request.endpoint,
        timeout,
        plan.read_request.strategy,
    );

    RuntimeTraceEntry {
        attempt,
        timestamp_ms,
        elapsed_ms,
        transport,
        result,
    }
}

fn read_runtime_trace_payload<T: UsbContext>(
    handle: &rusb::DeviceHandle<T>,
    buffer: &mut [u8],
    endpoint: u8,
    timeout: Duration,
    strategy: RuntimeReadStrategy,
) -> (Option<ReadTransport>, RuntimeTraceResult) {
    match strategy {
        RuntimeReadStrategy::Interrupt => match handle.read_interrupt(endpoint, buffer, timeout) {
            Ok(bytes_read) => (
                Some(ReadTransport::Interrupt),
                RuntimeTraceResult::Success(buffer[..bytes_read].to_vec()),
            ),
            Err(err) => (
                None,
                RuntimeTraceResult::Failure(format!("interrupt read error: {err}")),
            ),
        },
        RuntimeReadStrategy::Bulk => match handle.read_bulk(endpoint, buffer, timeout) {
            Ok(bytes_read) => (
                Some(ReadTransport::Bulk),
                RuntimeTraceResult::Success(buffer[..bytes_read].to_vec()),
            ),
            Err(err) => (
                None,
                RuntimeTraceResult::Failure(format!("bulk read error: {err}")),
            ),
        },
        RuntimeReadStrategy::Auto => {
            let interrupt_result = handle.read_interrupt(endpoint, buffer, timeout);
            match interrupt_result {
                Ok(bytes_read) => (
                    Some(ReadTransport::Interrupt),
                    RuntimeTraceResult::Success(buffer[..bytes_read].to_vec()),
                ),
                Err(interrupt_err) => match handle.read_bulk(endpoint, buffer, timeout) {
                    Ok(bytes_read) => (
                        Some(ReadTransport::Bulk),
                        RuntimeTraceResult::Success(buffer[..bytes_read].to_vec()),
                    ),
                    Err(bulk_err) => (
                        None,
                        RuntimeTraceResult::Failure(format!(
                            "interrupt read error: {interrupt_err}; bulk read error: {bulk_err}"
                        )),
                    ),
                },
            }
        }
    }
}

fn append_runtime_trace_entry(report: &mut String, entry: &RuntimeTraceEntry) {
    match &entry.result {
        RuntimeTraceResult::Success(bytes) => {
            let transport = entry
                .transport
                .map(read_transport_name)
                .unwrap_or("unknown transport");
            append_line(
                report,
                format!(
                    "    attempt {} t+{}ms ts={} {} {} byte(s): {}",
                    entry.attempt,
                    entry.elapsed_ms,
                    entry.timestamp_ms,
                    transport,
                    bytes.len(),
                    format_hex_bytes(bytes)
                ),
            );
        }
        RuntimeTraceResult::Failure(message) => {
            append_line(
                report,
                format!(
                    "    attempt {} t+{}ms ts={} read error: {}",
                    entry.attempt, entry.elapsed_ms, entry.timestamp_ms, message
                ),
            );
        }
    }
}

fn parse_usbmon_capture(input: &str) -> Result<Vec<UsbmonRecord>, String> {
    let mut records = Vec::new();

    for (index, line) in input.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let record = parse_usbmon_line(trimmed)
            .map_err(|err| format!("failed to parse usbmon line {}: {err}", index + 1))?;
        records.push(record);
    }

    Ok(records)
}

fn parse_usbmon_line(line: &str) -> Result<UsbmonRecord, String> {
    let tokens = line.split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 5 {
        return Err(format!("line is too short: {line}"));
    }

    let request_tag = tokens[0].to_string();
    let timestamp_us = tokens[1]
        .parse::<u64>()
        .map_err(|_| format!("invalid timestamp token: {}", tokens[1]))?;
    let event_kind = parse_usbmon_event_kind(tokens[2])?;
    let (transfer_type, direction, bus_number, device_address, endpoint) =
        parse_usbmon_address(tokens[3])?;
    let (status, length, payload_start_index) =
        parse_usbmon_status_and_length(&tokens, event_kind)?;
    let payload = parse_usbmon_payload(&tokens[payload_start_index..])?;
    let control_setup = parse_usbmon_control_setup(&tokens, event_kind, transfer_type)?;

    Ok(UsbmonRecord {
        request_tag,
        timestamp_us,
        event_kind,
        transfer_type,
        direction,
        bus_number,
        device_address,
        endpoint,
        status,
        length,
        payload,
        control_setup,
    })
}

fn parse_usbmon_status_and_length(
    tokens: &[&str],
    event_kind: UsbmonEventKind,
) -> Result<(i32, u32, usize), String> {
    match event_kind {
        UsbmonEventKind::Submit => {
            if tokens.len() < 5 {
                return Err(String::from("usbmon submit line is too short"));
            }

            let length_index = if let Some(payload_marker_index) = tokens
                .iter()
                .position(|token| *token == "=" || *token == "<")
            {
                payload_marker_index.saturating_sub(1)
            } else {
                tokens.len().saturating_sub(1)
            };
            let length_token = tokens
                .get(length_index)
                .ok_or_else(|| String::from("usbmon submit line is missing a length token"))?;
            let length = parse_usbmon_length_token(length_token)?;
            Ok((0, length, 5))
        }
        UsbmonEventKind::Complete | UsbmonEventKind::Error => {
            if tokens.len() < 6 {
                return Err(String::from("usbmon completion line is too short"));
            }

            let status = parse_usbmon_status_token(tokens[4])?;
            let length = parse_usbmon_length_token(tokens[5])?;
            Ok((status, length, 6))
        }
    }
}

fn parse_usbmon_status_token(token: &str) -> Result<i32, String> {
    let status_token = token.split(':').next().unwrap_or(token);
    status_token
        .parse::<i32>()
        .map_err(|_| format!("invalid status token: {token}"))
}

fn parse_usbmon_length_token(token: &str) -> Result<u32, String> {
    let length_token = token.split(':').next().unwrap_or(token);
    length_token
        .parse::<u32>()
        .map_err(|_| format!("invalid length token: {token}"))
}

fn parse_usbmon_event_kind(token: &str) -> Result<UsbmonEventKind, String> {
    match token {
        "S" => Ok(UsbmonEventKind::Submit),
        "C" => Ok(UsbmonEventKind::Complete),
        "E" => Ok(UsbmonEventKind::Error),
        other => Err(format!("unsupported usbmon event kind: {other}")),
    }
}

fn parse_usbmon_address(
    token: &str,
) -> Result<(CaptureTransferType, CaptureDirection, u16, u16, u8), String> {
    let mut parts = token.split(':');
    let prefix = parts
        .next()
        .ok_or_else(|| format!("missing usbmon address prefix: {token}"))?;
    let bus_number = parts
        .next()
        .ok_or_else(|| format!("missing usbmon bus number: {token}"))?
        .parse::<u16>()
        .map_err(|_| format!("invalid usbmon bus number: {token}"))?;
    let device_address = parts
        .next()
        .ok_or_else(|| format!("missing usbmon device address: {token}"))?
        .parse::<u16>()
        .map_err(|_| format!("invalid usbmon device address: {token}"))?;
    let endpoint = parts
        .next()
        .ok_or_else(|| format!("missing usbmon endpoint: {token}"))?
        .parse::<u8>()
        .map_err(|_| format!("invalid usbmon endpoint: {token}"))?;

    if parts.next().is_some() {
        return Err(format!("unexpected extra usbmon address fields: {token}"));
    }

    let prefix_bytes = prefix.as_bytes();
    if prefix_bytes.len() != 2 {
        return Err(format!("invalid usbmon address prefix: {token}"));
    }

    let transfer_type = match prefix_bytes[0] {
        b'B' => CaptureTransferType::Bulk,
        b'I' => CaptureTransferType::Interrupt,
        b'C' => CaptureTransferType::Control,
        b'Z' => CaptureTransferType::Isochronous,
        other => {
            return Err(format!(
                "unsupported usbmon transfer type: {}",
                other as char
            ));
        }
    };
    let direction = match prefix_bytes[1] {
        b'i' => CaptureDirection::In,
        b'o' => CaptureDirection::Out,
        other => return Err(format!("unsupported usbmon direction: {}", other as char)),
    };

    Ok((
        transfer_type,
        direction,
        bus_number,
        device_address,
        endpoint,
    ))
}

fn parse_usbmon_payload(tokens: &[&str]) -> Result<Vec<u8>, String> {
    let Some(payload_index) = tokens.iter().position(|token| *token == "=") else {
        return Ok(Vec::new());
    };

    let mut payload = Vec::new();
    for token in &tokens[payload_index + 1..] {
        if token.len() != 2 || !token.chars().all(|ch| ch.is_ascii_hexdigit()) {
            break;
        }
        payload.push(
            u8::from_str_radix(token, 16)
                .map_err(|_| format!("invalid usbmon payload byte: {token}"))?,
        );
    }

    Ok(payload)
}

fn parse_usbmon_control_setup(
    tokens: &[&str],
    event_kind: UsbmonEventKind,
    transfer_type: CaptureTransferType,
) -> Result<Option<UsbControlSetup>, String> {
    if event_kind != UsbmonEventKind::Submit || transfer_type != CaptureTransferType::Control {
        return Ok(None);
    }

    if tokens.get(4) != Some(&"s") {
        return Ok(None);
    }

    let Some(request_type) = tokens.get(5) else {
        return Ok(None);
    };
    let Some(request) = tokens.get(6) else {
        return Ok(None);
    };
    let Some(value) = tokens.get(7) else {
        return Ok(None);
    };
    let Some(index) = tokens.get(8) else {
        return Ok(None);
    };
    let Some(length) = tokens.get(9) else {
        return Ok(None);
    };

    Ok(Some(UsbControlSetup {
        request_type: parse_hex_u8_token(request_type)?,
        request: parse_hex_u8_token(request)?,
        value: parse_hex_u16_token(value)?,
        index: parse_hex_u16_token(index)?,
        length: parse_hex_u16_token(length)?,
    }))
}

fn filter_usbmon_records(
    records: &[UsbmonRecord],
    bus_number: Option<u16>,
    device_address: Option<u16>,
) -> Vec<UsbmonRecord> {
    records
        .iter()
        .filter(|record| bus_number.is_none_or(|bus| record.bus_number == bus))
        .filter(|record| device_address.is_none_or(|device| record.device_address == device))
        .cloned()
        .collect()
}

fn summarize_usbmon_records(records: &[UsbmonRecord]) -> UsbmonAnalysisSummary {
    let mut endpoints =
        BTreeMap::<(CaptureTransferType, CaptureDirection, u16, u16, u8), EndpointSummary>::new();
    let mut endpoint_timing = BTreeMap::<
        (CaptureTransferType, CaptureDirection, u16, u16, u8),
        EndpointTimingAccumulator,
    >::new();
    let mut outstanding_submits =
        BTreeMap::<String, ((CaptureTransferType, CaptureDirection, u16, u16, u8), u64)>::new();
    let mut control_requests = BTreeMap::<UsbControlSetup, usize>::new();

    for record in records {
        let key = (
            record.transfer_type,
            record.direction,
            record.bus_number,
            record.device_address,
            record.endpoint,
        );
        let summary = endpoints.entry(key).or_insert_with(|| EndpointSummary {
            transfer_type: record.transfer_type,
            direction: record.direction,
            bus_number: record.bus_number,
            device_address: record.device_address,
            endpoint: record.endpoint,
            total_records: 0,
            submit_count: 0,
            complete_count: 0,
            error_count: 0,
            length_counts: BTreeMap::new(),
            status_counts: BTreeMap::new(),
            submit_timing: None,
            completion_latency: None,
        });

        summary.total_records += 1;
        match record.event_kind {
            UsbmonEventKind::Submit => summary.submit_count += 1,
            UsbmonEventKind::Complete => summary.complete_count += 1,
            UsbmonEventKind::Error => summary.error_count += 1,
        }
        *summary.length_counts.entry(record.length).or_insert(0) += 1;
        *summary.status_counts.entry(record.status).or_insert(0) += 1;

        match record.event_kind {
            UsbmonEventKind::Submit => {
                let timing = endpoint_timing.entry(key).or_default();
                if let Some(last_submit_timestamp_us) = timing.last_submit_timestamp_us {
                    timing
                        .submit_gap
                        .observe(record.timestamp_us.saturating_sub(last_submit_timestamp_us));
                }
                timing.last_submit_timestamp_us = Some(record.timestamp_us);
                outstanding_submits.insert(record.request_tag.clone(), (key, record.timestamp_us));

                if let Some(setup) = record.control_setup {
                    *control_requests.entry(setup).or_insert(0) += 1;
                }
            }
            UsbmonEventKind::Complete | UsbmonEventKind::Error => {
                if let Some((submit_key, submit_timestamp_us)) =
                    outstanding_submits.remove(&record.request_tag)
                    && submit_key == key
                {
                    endpoint_timing
                        .entry(key)
                        .or_default()
                        .completion_latency
                        .observe(record.timestamp_us.saturating_sub(submit_timestamp_us));
                }
            }
        }
    }

    for (key, timing) in endpoint_timing {
        if let Some(summary) = endpoints.get_mut(&key) {
            summary.submit_timing = timing.submit_gap.finish();
            summary.completion_latency = timing.completion_latency.finish();
        }
    }

    UsbmonAnalysisSummary {
        total_records: records.len(),
        endpoint_summaries: endpoints.into_values().collect(),
        control_request_summaries: control_requests
            .into_iter()
            .map(|(setup, count)| ControlRequestSummary { setup, count })
            .collect(),
    }
}

#[derive(Debug, Default)]
struct EndpointTimingAccumulator {
    last_submit_timestamp_us: Option<u64>,
    submit_gap: TimingAccumulator,
    completion_latency: TimingAccumulator,
}

#[derive(Debug, Default)]
struct TimingAccumulator {
    sample_count: usize,
    total_us: u128,
    min_us: Option<u64>,
    max_us: Option<u64>,
}

impl TimingAccumulator {
    fn observe(&mut self, duration_us: u64) {
        self.sample_count += 1;
        self.total_us += u128::from(duration_us);
        self.min_us = Some(
            self.min_us
                .map_or(duration_us, |min_us| min_us.min(duration_us)),
        );
        self.max_us = Some(
            self.max_us
                .map_or(duration_us, |max_us| max_us.max(duration_us)),
        );
    }

    fn finish(&self) -> Option<TimingSummary> {
        if self.sample_count == 0 {
            return None;
        }

        let min_us = self.min_us?;
        let max_us = self.max_us?;
        let avg_us = (self.total_us / self.sample_count as u128) as u64;

        Some(TimingSummary {
            sample_count: self.sample_count,
            min_us,
            max_us,
            avg_us,
        })
    }
}

fn render_usbmon_analysis(
    summary: &UsbmonAnalysisSummary,
    options: &UsbmonAnalysisOptions,
) -> String {
    let mut report = String::new();
    append_line(&mut report, String::from("# usbmon Analysis"));
    append_blank_line(&mut report);
    append_line(&mut report, format!("Input: `{}`", options.input_path));
    if let Some(bus_number) = options.bus_number {
        append_line(&mut report, format!("Filtered bus: `{bus_number}`"));
    }
    if let Some(device_address) = options.device_address {
        append_line(
            &mut report,
            format!("Filtered device address: `{device_address}`"),
        );
    }
    append_line(
        &mut report,
        format!("Total parsed records: `{}`", summary.total_records),
    );

    append_blank_line(&mut report);
    append_line(&mut report, String::from("## Endpoint Activity"));
    append_blank_line(&mut report);

    if summary.endpoint_summaries.is_empty() {
        append_line(
            &mut report,
            String::from("- No usbmon records matched the requested filter."),
        );
    } else {
        for endpoint in &summary.endpoint_summaries {
            append_line(
                &mut report,
                format!(
                    "- Bus `{}` device `{}` endpoint `0x{:02x}` {} {}: records=`{}` submits=`{}` completes=`{}` errors=`{}` lengths=`{}` statuses=`{}`",
                    endpoint.bus_number,
                    endpoint.device_address,
                    effective_endpoint_address(
                        endpoint.endpoint,
                        endpoint.direction,
                        endpoint.transfer_type,
                    ),
                    capture_direction_name(endpoint.direction),
                    capture_transfer_type_name(endpoint.transfer_type),
                    endpoint.total_records,
                    endpoint.submit_count,
                    endpoint.complete_count,
                    endpoint.error_count,
                    render_count_map(&endpoint.length_counts),
                    render_count_map(&endpoint.status_counts)
                ),
            );
        }
    }

    append_blank_line(&mut report);
    append_line(&mut report, String::from("## Framing Hints"));
    append_blank_line(&mut report);
    if summary.endpoint_summaries.is_empty() {
        append_line(
            &mut report,
            String::from("- No framing hints are available without parsed records."),
        );
    } else {
        for endpoint in &summary.endpoint_summaries {
            append_line(
                &mut report,
                format!(
                    "- Endpoint `0x{:02x}` {} {} shows {} length pattern(s): {}",
                    effective_endpoint_address(
                        endpoint.endpoint,
                        endpoint.direction,
                        endpoint.transfer_type,
                    ),
                    capture_direction_name(endpoint.direction),
                    capture_transfer_type_name(endpoint.transfer_type),
                    endpoint.length_counts.len(),
                    describe_length_pattern(&endpoint.length_counts)
                ),
            );
        }
    }

    append_blank_line(&mut report);
    append_line(&mut report, String::from("## Timing Hints"));
    append_blank_line(&mut report);
    if summary.endpoint_summaries.is_empty() {
        append_line(
            &mut report,
            String::from("- No timing hints are available without parsed records."),
        );
    } else {
        let mut emitted_timing = false;
        for endpoint in &summary.endpoint_summaries {
            if let Some(timing) = endpoint.submit_timing {
                emitted_timing = true;
                append_line(
                    &mut report,
                    format!(
                        "- Endpoint `0x{:02x}` {} {} submit cadence: {}",
                        effective_endpoint_address(
                            endpoint.endpoint,
                            endpoint.direction,
                            endpoint.transfer_type,
                        ),
                        capture_direction_name(endpoint.direction),
                        capture_transfer_type_name(endpoint.transfer_type),
                        render_timing_summary(&timing)
                    ),
                );
            }
            if let Some(timing) = endpoint.completion_latency {
                emitted_timing = true;
                append_line(
                    &mut report,
                    format!(
                        "- Endpoint `0x{:02x}` {} {} completion latency: {}",
                        effective_endpoint_address(
                            endpoint.endpoint,
                            endpoint.direction,
                            endpoint.transfer_type,
                        ),
                        capture_direction_name(endpoint.direction),
                        capture_transfer_type_name(endpoint.transfer_type),
                        render_timing_summary(&timing)
                    ),
                );
            }
        }

        if !emitted_timing {
            append_line(
                &mut report,
                String::from("- No repeated submit/complete timing patterns were available."),
            );
        }
    }

    append_blank_line(&mut report);
    append_line(&mut report, String::from("## Control Requests"));
    append_blank_line(&mut report);
    if summary.control_request_summaries.is_empty() {
        append_line(
            &mut report,
            String::from("- No control setup packets were observed in the filtered capture."),
        );
    } else {
        for control_request in &summary.control_request_summaries {
            append_line(
                &mut report,
                format!(
                    "- {} count=`{}`",
                    describe_control_request(&control_request.setup),
                    control_request.count
                ),
            );
        }
    }

    append_blank_line(&mut report);
    append_line(&mut report, String::from("## Command/Response Hints"));
    append_blank_line(&mut report);
    append_line(
        &mut report,
        String::from("- Bulk OUT traffic on `0x01` is a likely command path when present."),
    );
    append_line(
        &mut report,
        String::from("- Bulk IN traffic on `0x81` is a likely response path when present."),
    );
    append_line(
        &mut report,
        String::from(
            "- Interrupt IN traffic on `0x83` is a likely status or event path when present.",
        ),
    );
    if summary
        .control_request_summaries
        .iter()
        .all(|request| !is_vendor_control_request(&request.setup))
    {
        append_line(
            &mut report,
            String::from(
                "- The observed control traffic is standard USB control flow so far, not a vendor-specific handshake.",
            ),
        );
    }

    append_blank_line(&mut report);
    append_line(&mut report, String::from("## Device Model Signals"));
    append_blank_line(&mut report);
    append_line(
        &mut report,
        render_device_model_signal(summary.endpoint_summaries.as_slice()),
    );

    report
}

fn render_usbmon_comparison(
    left: &UsbmonAnalysisSummary,
    right: &UsbmonAnalysisSummary,
    options: &UsbmonCompareOptions,
) -> String {
    let mut report = String::new();
    append_line(&mut report, String::from("# usbmon Comparison"));
    append_blank_line(&mut report);
    append_line(
        &mut report,
        format!("{}: `{}`", options.left_label, options.left_input_path),
    );
    append_line(
        &mut report,
        format!("{}: `{}`", options.right_label, options.right_input_path),
    );
    if let Some(bus_number) = options.bus_number {
        append_line(&mut report, format!("Filtered bus: `{bus_number}`"));
    }
    if let Some(device_address) = options.device_address {
        append_line(
            &mut report,
            format!("Filtered device address: `{device_address}`"),
        );
    }

    append_blank_line(&mut report);
    append_line(&mut report, String::from("## Totals"));
    append_blank_line(&mut report);
    append_line(
        &mut report,
        format!(
            "- Parsed records: {}=`{}`, {}=`{}`",
            options.left_label, left.total_records, options.right_label, right.total_records
        ),
    );

    append_blank_line(&mut report);
    append_line(&mut report, String::from("## Endpoint Differences"));
    append_blank_line(&mut report);

    let left_endpoints = left
        .endpoint_summaries
        .iter()
        .map(|summary| {
            (
                (
                    summary.transfer_type,
                    summary.direction,
                    summary.bus_number,
                    summary.device_address,
                    summary.endpoint,
                ),
                summary,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let right_endpoints = right
        .endpoint_summaries
        .iter()
        .map(|summary| {
            (
                (
                    summary.transfer_type,
                    summary.direction,
                    summary.bus_number,
                    summary.device_address,
                    summary.endpoint,
                ),
                summary,
            )
        })
        .collect::<BTreeMap<_, _>>();

    let endpoint_keys = left_endpoints
        .keys()
        .chain(right_endpoints.keys())
        .copied()
        .collect::<BTreeSet<_>>();

    if endpoint_keys.is_empty() {
        append_line(
            &mut report,
            String::from("- No endpoints were present in either filtered capture."),
        );
    } else {
        for key in endpoint_keys {
            let left_summary = left_endpoints.get(&key).copied();
            let right_summary = right_endpoints.get(&key).copied();
            let endpoint_address = effective_endpoint_address(key.4, key.1, key.0);
            let endpoint_name = format!(
                "endpoint `0x{:02x}` {} {}",
                endpoint_address,
                capture_direction_name(key.1),
                capture_transfer_type_name(key.0)
            );

            match (left_summary, right_summary) {
                (Some(left_summary), Some(right_summary)) => {
                    append_line(
                        &mut report,
                        format!(
                            "- {}: {} records=`{}` lengths=`{}` statuses=`{}`; {} records=`{}` lengths=`{}` statuses=`{}`",
                            endpoint_name,
                            options.left_label,
                            left_summary.total_records,
                            render_count_map(&left_summary.length_counts),
                            render_count_map(&left_summary.status_counts),
                            options.right_label,
                            right_summary.total_records,
                            render_count_map(&right_summary.length_counts),
                            render_count_map(&right_summary.status_counts),
                        ),
                    );
                }
                (Some(left_summary), None) => {
                    append_line(
                        &mut report,
                        format!(
                            "- {}: present only in {} with records=`{}` lengths=`{}` statuses=`{}`",
                            endpoint_name,
                            options.left_label,
                            left_summary.total_records,
                            render_count_map(&left_summary.length_counts),
                            render_count_map(&left_summary.status_counts),
                        ),
                    );
                }
                (None, Some(right_summary)) => {
                    append_line(
                        &mut report,
                        format!(
                            "- {}: present only in {} with records=`{}` lengths=`{}` statuses=`{}`",
                            endpoint_name,
                            options.right_label,
                            right_summary.total_records,
                            render_count_map(&right_summary.length_counts),
                            render_count_map(&right_summary.status_counts),
                        ),
                    );
                }
                (None, None) => {}
            }
        }
    }

    append_blank_line(&mut report);
    append_line(&mut report, String::from("## Control Request Differences"));
    append_blank_line(&mut report);

    let left_control_requests = left
        .control_request_summaries
        .iter()
        .map(|summary| (summary.setup, summary.count))
        .collect::<BTreeMap<_, _>>();
    let right_control_requests = right
        .control_request_summaries
        .iter()
        .map(|summary| (summary.setup, summary.count))
        .collect::<BTreeMap<_, _>>();
    let control_keys = left_control_requests
        .keys()
        .chain(right_control_requests.keys())
        .copied()
        .collect::<BTreeSet<_>>();

    if control_keys.is_empty() {
        append_line(
            &mut report,
            String::from("- No control setup packets were observed in either filtered capture."),
        );
    } else {
        for key in control_keys {
            let left_count = left_control_requests.get(&key).copied().unwrap_or(0);
            let right_count = right_control_requests.get(&key).copied().unwrap_or(0);
            append_line(
                &mut report,
                format!(
                    "- {}: {}=`{}`, {}=`{}`",
                    describe_control_request(&key),
                    options.left_label,
                    left_count,
                    options.right_label,
                    right_count
                ),
            );
        }
    }

    append_blank_line(&mut report);
    append_line(&mut report, String::from("## Comparison Hints"));
    append_blank_line(&mut report);
    append_line(
        &mut report,
        String::from(
            "- Look for endpoints or control requests present only in the working capture; those are strong candidates for the missing activation handshake.",
        ),
    );
    append_line(
        &mut report,
        String::from(
            "- A working capture that shows `0x01` OUT writes or non-empty `0x81`/`0x83` payloads would materially advance Phase 2.",
        ),
    );

    report
}

fn unix_timestamp_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_millis(0))
        .as_millis()
}

fn read_transport_name(transport: ReadTransport) -> &'static str {
    match transport {
        ReadTransport::Interrupt => "interrupt",
        ReadTransport::Bulk => "bulk",
    }
}

fn runtime_read_strategy_name(strategy: RuntimeReadStrategy) -> &'static str {
    match strategy {
        RuntimeReadStrategy::Auto => "auto",
        RuntimeReadStrategy::Interrupt => "interrupt",
        RuntimeReadStrategy::Bulk => "bulk",
    }
}

fn capture_direction_name(direction: CaptureDirection) -> &'static str {
    match direction {
        CaptureDirection::In => "IN",
        CaptureDirection::Out => "OUT",
    }
}

fn capture_transfer_type_name(transfer_type: CaptureTransferType) -> &'static str {
    match transfer_type {
        CaptureTransferType::Bulk => "bulk",
        CaptureTransferType::Interrupt => "interrupt",
        CaptureTransferType::Control => "control",
        CaptureTransferType::Isochronous => "isochronous",
    }
}

fn effective_endpoint_address(
    endpoint: u8,
    direction: CaptureDirection,
    transfer_type: CaptureTransferType,
) -> u8 {
    if transfer_type == CaptureTransferType::Control {
        return endpoint & 0x7f;
    }

    match direction {
        CaptureDirection::In => endpoint | 0x80,
        CaptureDirection::Out => endpoint & 0x7f,
    }
}

fn render_count_map<T: std::fmt::Display + Ord>(counts: &BTreeMap<T, usize>) -> String {
    counts
        .iter()
        .map(|(key, count)| format!("{key}:{count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn describe_length_pattern(length_counts: &BTreeMap<u32, usize>) -> String {
    if length_counts.is_empty() {
        return String::from("no lengths observed");
    }

    if length_counts.len() == 1 {
        let (&length, _) = length_counts
            .first_key_value()
            .expect("non-empty map should contain a length");
        return format!("fixed length `{length}`");
    }

    format!("variable lengths `{}`", render_count_map(length_counts))
}

fn render_device_model_signal(endpoint_summaries: &[EndpointSummary]) -> String {
    let max_bulk_in_length = endpoint_summaries
        .iter()
        .filter(|summary| {
            summary.transfer_type == CaptureTransferType::Bulk
                && summary.direction == CaptureDirection::In
        })
        .flat_map(|summary| summary.length_counts.keys())
        .copied()
        .max();

    match max_bulk_in_length {
        Some(length) if length >= 512 => String::from(
            "- Large bulk-IN payloads would be consistent with an image-oriented device path, but confirm with more than one capture before concluding that image data is being transferred.",
        ),
        Some(length) if length <= 64 => String::from(
            "- Only small payloads are visible so far; that leans toward status, template, or match-on-chip behavior, but the evidence is still too weak to classify the device model confidently.",
        ),
        Some(_) => String::from(
            "- Mid-sized payloads are visible, but they are not yet sufficient to distinguish between template-oriented and image-oriented behavior.",
        ),
        None => String::from(
            "- No bulk-IN payload evidence is available yet, so the device model remains unclassified.",
        ),
    }
}

fn render_timing_summary(summary: &TimingSummary) -> String {
    format!(
        "samples=`{}` avg=`{}` min=`{}` max=`{}`",
        summary.sample_count,
        format_duration_us(summary.avg_us),
        format_duration_us(summary.min_us),
        format_duration_us(summary.max_us)
    )
}

fn format_duration_us(duration_us: u64) -> String {
    let whole_ms = duration_us / 1_000;
    let fractional_ms = duration_us % 1_000;
    format!("{whole_ms}.{fractional_ms:03}ms")
}

fn describe_control_request(setup: &UsbControlSetup) -> String {
    format!(
        "{} {} {} {} wValue=`0x{:04x}` wIndex=`0x{:04x}` wLength=`{}`",
        control_request_type_name(setup.request_type),
        control_recipient_name(setup.request_type),
        control_direction_name(setup.request_type),
        control_request_name(setup.request_type, setup.request),
        setup.value,
        setup.index,
        setup.length
    )
}

fn control_direction_name(request_type: u8) -> &'static str {
    if request_type & 0x80 != 0 {
        "IN"
    } else {
        "OUT"
    }
}

fn control_request_type_name(request_type: u8) -> &'static str {
    match (request_type >> 5) & 0x03 {
        0 => "standard",
        1 => "class",
        2 => "vendor",
        _ => "reserved",
    }
}

fn control_recipient_name(request_type: u8) -> &'static str {
    match request_type & 0x1f {
        0 => "device",
        1 => "interface",
        2 => "endpoint",
        3 => "other",
        _ => "unknown",
    }
}

fn control_request_name(request_type: u8, request: u8) -> String {
    if ((request_type >> 5) & 0x03) != 0 {
        return format!("request `0x{request:02x}`");
    }

    let name = match request {
        0x00 => "GET_STATUS",
        0x01 => "CLEAR_FEATURE",
        0x03 => "SET_FEATURE",
        0x05 => "SET_ADDRESS",
        0x06 => "GET_DESCRIPTOR",
        0x07 => "SET_DESCRIPTOR",
        0x08 => "GET_CONFIGURATION",
        0x09 => "SET_CONFIGURATION",
        0x0a => "GET_INTERFACE",
        0x0b => "SET_INTERFACE",
        0x0c => "SYNCH_FRAME",
        _ => return format!("request `0x{request:02x}`"),
    };

    String::from(name)
}

fn is_vendor_control_request(setup: &UsbControlSetup) -> bool {
    ((setup.request_type >> 5) & 0x03) == 0x02
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
    println!("  runtime-trace          Repeated bounded runtime reads with timestamps");
    println!(
        "  runtime-trace --claim 0 --read-ep 0x83 --iterations 8 --output captures/runtime-trace.txt"
    );
    println!(
        "  runtime-trace --claim 0 --read-ep 0x81 --transport bulk --iterations 8 --output captures/runtime-trace-bulk.txt"
    );
    println!("  analyze-usbmon         Summarize a usbmon text capture");
    println!(
        "  analyze-usbmon --input captures/usbmon-20260326/usbmon-bus1.txt --output captures/usbmon-analysis.md"
    );
    println!("  compare-usbmon         Compare two usbmon text captures");
    println!(
        "  compare-usbmon --left captures/linux.txt --right captures/reference.txt --bus 1 --device 3 --output captures/usbmon-compare.md"
    );
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
    RuntimeTrace(RuntimeTraceOptions),
    AnalyzeUsbmon(UsbmonAnalysisOptions),
    CompareUsbmon(UsbmonCompareOptions),
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

#[derive(Debug, PartialEq, Eq)]
struct RuntimeTraceOptions {
    vendor_id: u16,
    product_id: u16,
    output_path: Option<String>,
    claim_interface: u8,
    read_endpoint: u8,
    read_strategy: RuntimeReadStrategy,
    read_length: u16,
    timeout_ms: u64,
    iterations: u16,
    delay_ms: u64,
}

#[derive(Debug, PartialEq, Eq)]
struct UsbmonAnalysisOptions {
    input_path: String,
    output_path: Option<String>,
    bus_number: Option<u16>,
    device_address: Option<u16>,
}

#[derive(Debug, PartialEq, Eq)]
struct UsbmonCompareOptions {
    left_input_path: String,
    right_input_path: String,
    output_path: Option<String>,
    left_label: String,
    right_label: String,
    bus_number: Option<u16>,
    device_address: Option<u16>,
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
    strategy: RuntimeReadStrategy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RuntimeTracePlan {
    interface: u8,
    read_request: BoundedReadRequest,
    iterations: u16,
    delay_ms: u64,
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

impl Default for RuntimeTraceOptions {
    fn default() -> Self {
        Self {
            vendor_id: DEFAULT_VENDOR_ID,
            product_id: DEFAULT_PRODUCT_ID,
            output_path: None,
            claim_interface: 0,
            read_endpoint: 0x83,
            read_strategy: RuntimeReadStrategy::Auto,
            read_length: 64,
            timeout_ms: 250,
            iterations: 8,
            delay_ms: 0,
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
            "runtime-trace" => parse_runtime_trace_args(&tokens[1..]),
            "analyze-usbmon" => parse_analyze_usbmon_args(&tokens[1..]),
            "compare-usbmon" => parse_compare_usbmon_args(&tokens[1..]),
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

fn parse_runtime_trace_args(tokens: &[String]) -> Result<Args, String> {
    let mut options = RuntimeTraceOptions::default();

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
                options.claim_interface = parse_u8(value, "--claim")?;
                index += 2;
            }
            "--read-ep" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --read-ep"))?;
                options.read_endpoint = parse_hex_u8(value)?;
                index += 2;
            }
            "--transport" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --transport"))?;
                options.read_strategy = parse_runtime_read_strategy(value)?;
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
            "--iterations" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --iterations"))?;
                options.iterations = parse_u16(value, "--iterations")?;
                index += 2;
            }
            "--delay-ms" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --delay-ms"))?;
                options.delay_ms = parse_u64(value, "--delay-ms")?;
                index += 2;
            }
            "--help" | "-h" => {
                return Ok(Args {
                    command: Command::List,
                });
            }
            other => return Err(format!("unknown runtime-trace option: {other}")),
        }
    }

    if options.iterations == 0 {
        return Err(String::from(
            "--iterations must be greater than zero for runtime tracing",
        ));
    }

    build_runtime_trace_plan(&options)?;

    Ok(Args {
        command: Command::RuntimeTrace(options),
    })
}

fn parse_analyze_usbmon_args(tokens: &[String]) -> Result<Args, String> {
    let mut options = UsbmonAnalysisOptions {
        input_path: String::new(),
        output_path: None,
        bus_number: None,
        device_address: None,
    };

    let mut index = 0;
    while index < tokens.len() {
        match tokens[index].as_str() {
            "--input" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --input"))?;
                options.input_path = value.clone();
                index += 2;
            }
            "--output" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --output"))?;
                options.output_path = Some(value.clone());
                index += 2;
            }
            "--bus" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --bus"))?;
                options.bus_number = Some(parse_u16(value, "--bus")?);
                index += 2;
            }
            "--device" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --device"))?;
                options.device_address = Some(parse_u16(value, "--device")?);
                index += 2;
            }
            "--help" | "-h" => {
                return Ok(Args {
                    command: Command::List,
                });
            }
            other => return Err(format!("unknown analyze-usbmon option: {other}")),
        }
    }

    if options.input_path.is_empty() {
        return Err(String::from(
            "--input is required so analyze-usbmon knows which capture to parse",
        ));
    }

    Ok(Args {
        command: Command::AnalyzeUsbmon(options),
    })
}

fn parse_compare_usbmon_args(tokens: &[String]) -> Result<Args, String> {
    let mut options = UsbmonCompareOptions {
        left_input_path: String::new(),
        right_input_path: String::new(),
        output_path: None,
        left_label: String::from("left"),
        right_label: String::from("right"),
        bus_number: None,
        device_address: None,
    };

    let mut index = 0;
    while index < tokens.len() {
        match tokens[index].as_str() {
            "--left" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --left"))?;
                options.left_input_path = value.clone();
                index += 2;
            }
            "--right" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --right"))?;
                options.right_input_path = value.clone();
                index += 2;
            }
            "--left-label" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --left-label"))?;
                options.left_label = value.clone();
                index += 2;
            }
            "--right-label" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --right-label"))?;
                options.right_label = value.clone();
                index += 2;
            }
            "--output" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --output"))?;
                options.output_path = Some(value.clone());
                index += 2;
            }
            "--bus" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --bus"))?;
                options.bus_number = Some(parse_u16(value, "--bus")?);
                index += 2;
            }
            "--device" => {
                let value = tokens
                    .get(index + 1)
                    .ok_or_else(|| String::from("missing value for --device"))?;
                options.device_address = Some(parse_u16(value, "--device")?);
                index += 2;
            }
            "--help" | "-h" => {
                return Ok(Args {
                    command: Command::List,
                });
            }
            other => return Err(format!("unknown compare-usbmon option: {other}")),
        }
    }

    if options.left_input_path.is_empty() {
        return Err(String::from(
            "--left is required so compare-usbmon knows the first capture to parse",
        ));
    }
    if options.right_input_path.is_empty() {
        return Err(String::from(
            "--right is required so compare-usbmon knows the second capture to parse",
        ));
    }

    Ok(Args {
        command: Command::CompareUsbmon(options),
    })
}

fn parse_hex_u16(value: &str) -> Result<u16, String> {
    parse_hex_u16_token(value)
}

fn parse_hex_u8(value: &str) -> Result<u8, String> {
    parse_hex_u8_token(value)
}

fn parse_hex_u16_token(value: &str) -> Result<u16, String> {
    let trimmed = value.trim_start_matches("0x");
    u16::from_str_radix(trimmed, 16).map_err(|_| format!("invalid hex value: {value}"))
}

fn parse_hex_u8_token(value: &str) -> Result<u8, String> {
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

fn parse_runtime_read_strategy(value: &str) -> Result<RuntimeReadStrategy, String> {
    match value {
        "auto" => Ok(RuntimeReadStrategy::Auto),
        "interrupt" => Ok(RuntimeReadStrategy::Interrupt),
        "bulk" => Ok(RuntimeReadStrategy::Bulk),
        other => Err(format!(
            "invalid value for --transport: {other}; expected auto, interrupt, or bulk"
        )),
    }
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
        Args, BoundedReadRequest, CaptureDirection, CaptureTransferType, Command,
        ExecutionEnvironment, ProbeOptions, RuntimeProbePlan, RuntimeReadStrategy,
        RuntimeTraceOptions, UsbmonAnalysisOptions, UsbmonCompareOptions, build_runtime_probe_plan,
        build_runtime_trace_plan, parse_hex_u16, parse_usbmon_capture, parse_usbmon_line,
        render_device_not_found_message_from_visible_devices, render_usbmon_comparison,
        summarize_usbmon_records,
    };

    #[test]
    fn parses_default_probe_command() {
        let args = Args::parse(Vec::new()).expect("default args should parse");
        match args.command {
            Command::Probe(options) => assert_eq!(options, ProbeOptions::default()),
            Command::DeviceProfile(_) => panic!("expected probe command"),
            Command::RuntimeTrace(_) => panic!("expected probe command"),
            Command::AnalyzeUsbmon(_) => panic!("expected probe command"),
            Command::CompareUsbmon(_) => panic!("expected probe command"),
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
            Command::RuntimeTrace(_) => panic!("expected probe command"),
            Command::AnalyzeUsbmon(_) => panic!("expected probe command"),
            Command::CompareUsbmon(_) => panic!("expected probe command"),
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
            Command::RuntimeTrace(_) => panic!("expected probe command"),
            Command::AnalyzeUsbmon(_) => panic!("expected probe command"),
            Command::CompareUsbmon(_) => panic!("expected probe command"),
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
            Command::RuntimeTrace(_) => panic!("expected device-profile command"),
            Command::AnalyzeUsbmon(_) => panic!("expected device-profile command"),
            Command::CompareUsbmon(_) => panic!("expected device-profile command"),
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
                    strategy: RuntimeReadStrategy::Auto,
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

    #[test]
    fn parses_runtime_trace_command() {
        let tokens = vec![
            String::from("runtime-trace"),
            String::from("--claim"),
            String::from("0"),
            String::from("--read-ep"),
            String::from("0x83"),
            String::from("--transport"),
            String::from("interrupt"),
            String::from("--iterations"),
            String::from("4"),
            String::from("--delay-ms"),
            String::from("10"),
        ];

        let args = Args::parse(tokens).expect("runtime-trace args should parse");

        match args.command {
            Command::RuntimeTrace(options) => {
                assert_eq!(options.claim_interface, 0);
                assert_eq!(options.read_endpoint, 0x83);
                assert_eq!(options.read_strategy, RuntimeReadStrategy::Interrupt);
                assert_eq!(options.iterations, 4);
                assert_eq!(options.delay_ms, 10);
            }
            Command::Probe(_) => panic!("expected runtime-trace command"),
            Command::DeviceProfile(_) => panic!("expected runtime-trace command"),
            Command::AnalyzeUsbmon(_) => panic!("expected runtime-trace command"),
            Command::CompareUsbmon(_) => panic!("expected runtime-trace command"),
            Command::List => panic!("expected runtime-trace command"),
        }
    }

    #[test]
    fn runtime_trace_plan_rejects_out_endpoint_reads() {
        let options = RuntimeTraceOptions {
            read_endpoint: 0x01,
            ..RuntimeTraceOptions::default()
        };

        let error = build_runtime_trace_plan(&options)
            .expect_err("runtime trace should reject OUT endpoints");

        assert!(error.contains("endpoint 0x01 is not an IN endpoint"));
    }

    #[test]
    fn runtime_trace_plan_preserves_bulk_transport_strategy() {
        let options = RuntimeTraceOptions {
            read_endpoint: 0x81,
            read_strategy: RuntimeReadStrategy::Bulk,
            ..RuntimeTraceOptions::default()
        };

        let plan = build_runtime_trace_plan(&options).expect("runtime trace plan should build");

        assert_eq!(plan.read_request.endpoint, 0x81);
        assert_eq!(plan.read_request.strategy, RuntimeReadStrategy::Bulk);
    }

    #[test]
    fn parses_analyze_usbmon_command() {
        let tokens = vec![
            String::from("analyze-usbmon"),
            String::from("--input"),
            String::from("captures/usbmon.txt"),
            String::from("--bus"),
            String::from("1"),
            String::from("--device"),
            String::from("3"),
        ];

        let args = Args::parse(tokens).expect("analyze-usbmon args should parse");

        match args.command {
            Command::AnalyzeUsbmon(UsbmonAnalysisOptions {
                input_path,
                bus_number,
                device_address,
                ..
            }) => {
                assert_eq!(input_path, "captures/usbmon.txt");
                assert_eq!(bus_number, Some(1));
                assert_eq!(device_address, Some(3));
            }
            Command::Probe(_) => panic!("expected analyze-usbmon command"),
            Command::DeviceProfile(_) => panic!("expected analyze-usbmon command"),
            Command::RuntimeTrace(_) => panic!("expected analyze-usbmon command"),
            Command::CompareUsbmon(_) => panic!("expected analyze-usbmon command"),
            Command::List => panic!("expected analyze-usbmon command"),
        }
    }

    #[test]
    fn parses_compare_usbmon_command() {
        let tokens = vec![
            String::from("compare-usbmon"),
            String::from("--left"),
            String::from("captures/linux.txt"),
            String::from("--right"),
            String::from("captures/reference.txt"),
            String::from("--left-label"),
            String::from("linux"),
            String::from("--right-label"),
            String::from("reference"),
            String::from("--bus"),
            String::from("1"),
            String::from("--device"),
            String::from("3"),
        ];

        let args = Args::parse(tokens).expect("compare-usbmon args should parse");

        match args.command {
            Command::CompareUsbmon(options) => {
                assert_eq!(options.left_input_path, "captures/linux.txt");
                assert_eq!(options.right_input_path, "captures/reference.txt");
                assert_eq!(options.left_label, "linux");
                assert_eq!(options.right_label, "reference");
                assert_eq!(options.bus_number, Some(1));
                assert_eq!(options.device_address, Some(3));
            }
            Command::Probe(_) => panic!("expected compare-usbmon command"),
            Command::DeviceProfile(_) => panic!("expected compare-usbmon command"),
            Command::RuntimeTrace(_) => panic!("expected compare-usbmon command"),
            Command::AnalyzeUsbmon(_) => panic!("expected compare-usbmon command"),
            Command::List => panic!("expected compare-usbmon command"),
        }
    }

    #[test]
    fn parses_usbmon_completion_line() {
        let record = parse_usbmon_line(
            "ffff9f0c7d4c6e00 1234567890 C Ii:1:003:3 0 8 = 11 22 33 44 55 66 77 88",
        )
        .expect("usbmon line should parse");

        assert_eq!(record.transfer_type, CaptureTransferType::Interrupt);
        assert_eq!(record.direction, CaptureDirection::In);
        assert_eq!(record.bus_number, 1);
        assert_eq!(record.device_address, 3);
        assert_eq!(record.endpoint, 3);
        assert_eq!(record.length, 8);
        assert_eq!(record.payload.len(), 8);
    }

    #[test]
    fn parses_usbmon_control_submit_line() {
        let record =
            parse_usbmon_line("ffff8cfc4f6110c0 3903091966 S Co:1:001:0 s 23 03 0002 0007 0000 0")
                .expect("control submit line should parse");

        assert_eq!(record.transfer_type, CaptureTransferType::Control);
        assert_eq!(record.direction, CaptureDirection::Out);
        assert_eq!(record.bus_number, 1);
        assert_eq!(record.device_address, 1);
        assert_eq!(record.endpoint, 0);
        assert_eq!(record.status, 0);
        assert_eq!(record.length, 0);
        assert_eq!(record.timestamp_us, 3_903_091_966);
        assert_eq!(
            record
                .control_setup
                .as_ref()
                .map(|setup| setup.request_type),
            Some(0x23)
        );
        assert_eq!(
            record.control_setup.as_ref().map(|setup| setup.request),
            Some(0x03)
        );
        assert_eq!(
            record.control_setup.as_ref().map(|setup| setup.value),
            Some(0x0002)
        );
        assert_eq!(
            record.control_setup.as_ref().map(|setup| setup.index),
            Some(0x0007)
        );
        assert_eq!(
            record.control_setup.as_ref().map(|setup| setup.length),
            Some(0x0000)
        );
    }

    #[test]
    fn parses_usbmon_control_submit_line_with_input_marker() {
        let record =
            parse_usbmon_line("ffff8cfc4f6110c0 118540612 S Ci:1:001:0 s a3 00 0000 0001 0004 4 <")
                .expect("control submit line with input marker should parse");

        assert_eq!(record.transfer_type, CaptureTransferType::Control);
        assert_eq!(record.direction, CaptureDirection::In);
        assert_eq!(record.bus_number, 1);
        assert_eq!(record.device_address, 1);
        assert_eq!(record.endpoint, 0);
        assert_eq!(record.status, 0);
        assert_eq!(record.length, 4);
        assert_eq!(
            record
                .control_setup
                .as_ref()
                .map(|setup| setup.request_type),
            Some(0xa3)
        );
        assert_eq!(
            record.control_setup.as_ref().map(|setup| setup.request),
            Some(0x00)
        );
        assert_eq!(
            record.control_setup.as_ref().map(|setup| setup.value),
            Some(0x0000)
        );
        assert_eq!(
            record.control_setup.as_ref().map(|setup| setup.index),
            Some(0x0001)
        );
        assert_eq!(
            record.control_setup.as_ref().map(|setup| setup.length),
            Some(0x0004)
        );
    }

    #[test]
    fn parses_usbmon_completion_line_with_status_suffix() {
        let record = parse_usbmon_line("ffff8cfc44500cc0 3903114072 C Ii:1:001:1 -2:2048 0")
            .expect("completion line with status suffix should parse");

        assert_eq!(record.transfer_type, CaptureTransferType::Interrupt);
        assert_eq!(record.direction, CaptureDirection::In);
        assert_eq!(record.bus_number, 1);
        assert_eq!(record.device_address, 1);
        assert_eq!(record.endpoint, 1);
        assert_eq!(record.status, -2);
        assert_eq!(record.length, 0);
    }

    #[test]
    fn summarizes_usbmon_records_by_endpoint() {
        let capture = "\
ffff9f0c7d4c6e00 1234567000 S Ci:1:003:0 s 80 00 0000 0000 0002 2 <\n\
ffff9f0c7d4c6e00 1234567100 C Ci:1:003:0 0 2 = 0000\n\
ffff9f0c7d4c6e01 1234567200 S Ii:1:003:3 -115:4 64 <\n\
ffff9f0c7d4c6e01 1234567450 C Ii:1:003:3 -2:4 0\n\
ffff9f0c7d4c6e02 1234567700 S Ii:1:003:3 -115:4 64 <\n\
ffff9f0c7d4c6e02 1234567950 C Ii:1:003:3 -2:4 0\n";
        let records = parse_usbmon_capture(capture).expect("capture should parse");
        let summary = summarize_usbmon_records(&records);

        assert_eq!(summary.total_records, 6);
        assert_eq!(summary.endpoint_summaries.len(), 2);
        assert_eq!(summary.control_request_summaries.len(), 1);
        assert_eq!(summary.control_request_summaries[0].count, 1);
        assert_eq!(
            summary.control_request_summaries[0].setup.request_type,
            0x80
        );
        assert_eq!(summary.control_request_summaries[0].setup.request, 0x00);
        assert!(
            summary
                .endpoint_summaries
                .iter()
                .any(|endpoint| endpoint.endpoint == 0 && endpoint.total_records == 2)
        );
        let interrupt_endpoint = summary
            .endpoint_summaries
            .iter()
            .find(|endpoint| endpoint.endpoint == 3)
            .expect("interrupt endpoint summary should exist");
        assert_eq!(interrupt_endpoint.total_records, 4);
        assert_eq!(
            interrupt_endpoint
                .submit_timing
                .as_ref()
                .map(|timing| timing.sample_count),
            Some(1)
        );
        assert_eq!(
            interrupt_endpoint
                .submit_timing
                .as_ref()
                .map(|timing| timing.avg_us),
            Some(500)
        );
        assert_eq!(
            interrupt_endpoint
                .completion_latency
                .as_ref()
                .map(|timing| timing.avg_us),
            Some(250)
        );
    }

    #[test]
    fn renders_usbmon_comparison_with_endpoint_difference() {
        let left_capture = "\
ffff9f0c7d4c6e00 1234567000 S Ii:1:003:3 -115:4 64 <\n\
ffff9f0c7d4c6e00 1234567250 C Ii:1:003:3 -2:4 0\n";
        let right_capture = "\
ffff9f0c7d4c6e00 1234567000 S Ii:1:003:3 -115:4 64 <\n\
ffff9f0c7d4c6e00 1234567250 C Ii:1:003:3 0:4 8 = 11 22 33 44 55 66 77 88\n\
ffff9f0c7d4c6e00 1234567300 S Bo:1:003:1 0 64 = aa bb\n\
ffff9f0c7d4c6e00 1234567350 C Bo:1:003:1 0 64\n";
        let left_summary =
            summarize_usbmon_records(&parse_usbmon_capture(left_capture).expect("left parse"));
        let right_summary =
            summarize_usbmon_records(&parse_usbmon_capture(right_capture).expect("right parse"));

        let report = render_usbmon_comparison(
            &left_summary,
            &right_summary,
            &UsbmonCompareOptions {
                left_input_path: String::from("captures/linux.txt"),
                right_input_path: String::from("captures/reference.txt"),
                output_path: None,
                left_label: String::from("linux"),
                right_label: String::from("reference"),
                bus_number: Some(1),
                device_address: Some(3),
            },
        );

        assert!(report.contains("endpoint `0x83` IN interrupt"));
        assert!(report.contains("endpoint `0x01` OUT bulk: present only in reference"));
        assert!(report.contains("working capture"));
    }
}
