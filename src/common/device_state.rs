// SPDX-License-Identifier: CC0-1.0
// Public-domain example code (CC0) - see ../../LICENSE.

//! device_state.rs
//! =========================================================================
//! The device's mutable state, in one place.
//!
//! THIS IS THE SINGLE MOST IMPORTANT DESIGN POINT IN THIS EXAMPLE. The
//! `BACnetStack_RegisterCallbackGetProperty*` functions take
//! `Option<extern "C" fn(...)>` - plain, non-capturing function pointers,
//! not closures. A `Get*Property` callback therefore CANNOT close over any
//! `main()`-local variable the way the C++ edition's lambdas or the Python
//! edition's closures do; it can only read module-level state.
//!
//! So every piece of device state a callback needs to read (the Analog
//! Input's live value, the Network Port's discovered IP addressing, the
//! configured device instance and UDP port) lives in this one
//! `once_cell::sync::Lazy<Mutex<DeviceState>>` static, and each
//! `extern "C" fn CallbackGetProperty*` in `main.rs` locks it and reads from
//! it. `main()` also writes through this same lock (on start-up, to record
//! the resolved IP addressing; in the interactive-key loop, to nudge the
//! Analog Input).
//!
//! A `std::sync::Mutex` is correct here (not e.g. an atomic-per-field
//! scheme) because the CAS BACnet Stack is single-threaded by contract -
//! nothing in this example spawns a thread that touches the stack, so the
//! lock is never contended. It exists to satisfy Rust's `Sync` requirement
//! for a `static`, not to serialize concurrent access.
//! =========================================================================

use once_cell::sync::Lazy;
use std::sync::Mutex;

/// Everything a `Get*Property` callback (or `main()`) needs to read or write
/// after start-up. Fields fixed for the life of the process (the colour
/// names, the Multi-State Input's state count) are plain `const`s in
/// `main.rs` instead of living here - only things that are genuinely
/// runtime-configurable or mutable belong in this struct.
pub struct DeviceState {
    /// The BACnet Device instance (`--deviceID`, default 389001).
    pub device_instance: u32,
    /// The BACnet/IP UDP port this device is listening on (`--port`).
    pub bacnet_ip_udp_port: u16,

    /// Analog Input 1's live present value (degrees Celsius). Starts at 21.5
    /// and is nudged by the up/down arrow keys. A real sensor would update
    /// this from hardware instead.
    pub analog_input_1_value: f32,

    /// BACnet/IP addressing the Network Port reports - filled in at
    /// start-up from the host's primary interface. The gateway is left
    /// unset (0.0.0.0) for this example.
    pub ip_address: [u8; 4],
    pub ip_subnet_mask: [u8; 4],
    pub ip_default_gateway: [u8; 4],

    /// The Device object's Firmware_Revision (property 44) - the underlying
    /// CAS BACnet Stack's own version, NOT this example's version. Built
    /// once at start-up (in `main()`, right after the native library loads
    /// successfully) from `BACnetStack_GetAPIMajorVersion`/`MinorVersion`/
    /// `PatchVersion`/`BuildVersion` - the same four calls
    /// `cas_example_helper::print_version` already uses for the start-up
    /// banner. Empty until then; a `Get*Property` callback should never
    /// observe it empty in practice because `main()` sets it before
    /// registering any callback or adding the Device object.
    pub firmware_revision: String,
}

impl DeviceState {
    fn new() -> Self {
        DeviceState {
            device_instance: 389001,
            bacnet_ip_udp_port: 47808,
            analog_input_1_value: 21.5,
            ip_address: [0, 0, 0, 0],
            ip_subnet_mask: [0, 0, 0, 0],
            ip_default_gateway: [0, 0, 0, 0],
            firmware_revision: String::new(),
        }
    }
}

/// The one instance of device state, reachable from every free-standing
/// `extern "C" fn` callback as well as from `main()`. See the module doc
/// above for why this has to be a static rather than a captured variable.
pub static STATE: Lazy<Mutex<DeviceState>> = Lazy::new(|| Mutex::new(DeviceState::new()));
