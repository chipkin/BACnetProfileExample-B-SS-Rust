// SPDX-License-Identifier: CC0-1.0
// Public-domain example code (CC0) - see ../../LICENSE.

//! cas_example_helper.rs
//! =========================================================================
//! Shared plumbing for the BACnet examples - the Rust edition of the C++
//! examples' `common/CASExampleHelper.{h,cpp}` (and the C#/Python editions'
//! `common/CASExampleHelper.cs` / `common/cas_example_helper.py`), same
//! responsibilities, same names:
//!
//!   - `register_common_callbacks()` - the three transport/time callbacks
//!     every example needs (receive, send, system time). This is where the
//!     6-byte IPv4 connection string lives: 4 IP octets then the port in
//!     BIG-endian byte order - written here ONCE so no example re-derives
//!     it.
//!   - `send_i_am()` - the unsolicited I-Am every example transmits on
//!     start-up.
//!   - `get_local_ipv4()` - the primary interface's address/netmask/
//!     broadcast (the OS-level plumbing the Network Port object reports).
//!   - CLI helpers (`--deviceID`/`--port` parsing) + `print_version()`.
//!
//! The stack PULLS datagrams: its receive callback asks for one queued
//! datagram per call and the stack never touches the socket. The
//! application (`SimpleUdp`) owns the socket - keep that split in your own
//! project.
//!
//! PLAIN FUNCTION POINTERS, NOT CLOSURES. `BACnetStack_RegisterCallback*`
//! takes `Option<extern "C" fn(...)>` - a bare, non-capturing function
//! pointer. The three callbacks below (`on_receive`, `on_send`,
//! `on_get_system_time`) are therefore free-standing `extern "C" fn` items,
//! not closures over `register_common_callbacks()`'s own locals, and they
//! reach the one UDP socket and the Network Port instance through the
//! module-level statics below - the same pattern `device_state.rs`
//! documents for the property callbacks in `main.rs`.
//!
//! PANIC SAFETY. Each callback wraps its body in `catch_unwind` and returns
//! the "did nothing" value (0 for the transport callbacks) on a caught
//! panic, rather than letting it unwind across the `extern "C"` boundary
//! into the native stack - undefined behaviour otherwise. See AGENTS.md.
//!
//! PORT-KEYED TRANSPORT (this example's stack pin, branch `6.x`). A link is
//! identified by the Network Port object's INSTANCE, not by a
//! transport-type enumeration: `BACnetStack_RegisterCallbackReceiveMessageForPort`
//! / `BACnetStack_RegisterCallbackSendMessageForPort` take/report a
//! `networkPortInstance`, and `BACnetStack_SendIAm`'s fourth argument is
//! that same instance. This example has exactly one Network Port
//! (`constants::NETWORK_PORT_INSTANCE`, "Vermilion"), so every call below
//! names it directly rather than looping over a port table.
//! =========================================================================

use std::net::Ipv4Addr;
use std::panic::catch_unwind;
use std::sync::Mutex;

use once_cell::sync::Lazy;

use crate::cas_bacnet_stack::cas_bacnet_stack_adapters as bacnet;
use crate::common::constants;
use crate::common::simple_udp::SimpleUdp;

/// The version of the vendored `common/` helper itself (NOT the example's
/// own version). Bump it whenever anything in `common/` changes, and record
/// the change in `common/CHANGELOG.md`. Printed by `print_version()` below.
pub const COMMON_VERSION: &str = "1.0.0";

/// The one UDP socket this example owns, reachable from the free-standing
/// `extern "C" fn` transport callbacks below - see the module doc's "plain
/// function pointers, not closures" note.
static UDP: Lazy<Mutex<SimpleUdp>> = Lazy::new(|| Mutex::new(SimpleUdp::new()));

/// Bind the shared UDP socket. Call once, before `register_common_callbacks`.
pub fn setup_udp(port: u16) -> std::io::Result<()> {
    UDP.lock().unwrap().setup(port)
}

/// Close the shared UDP socket.
pub fn shutdown_udp() {
    UDP.lock().unwrap().shutdown();
}

// -----------------------------------------------------------------------------
// Local IPv4 discovery
// -----------------------------------------------------------------------------

pub struct LocalIpv4 {
    pub address: Ipv4Addr,
    pub netmask: Ipv4Addr,
    pub broadcast: Ipv4Addr,
}

/// The primary IPv4 interface's address, guessed netmask, and derived
/// broadcast address. The Network Port object reports these values, and
/// `send_i_am()` targets the derived subnet broadcast.
///
/// DEVIATION FROM THE C++/C# EDITIONS (matches the Python edition's own
/// documented deviation): those use OS-specific interface enumeration
/// (`getifaddrs` / `NetworkInterface.GetAllNetworkInterfaces`) to read the
/// REAL subnet mask. Rust's standard library has no portable equivalent (a
/// real one needs a third-party crate such as `if-addrs`, which this example
/// deliberately avoids to keep the dependency list to exactly
/// `libloading`/`once_cell`/`ctrlc` - see Cargo.toml). Instead this function
/// opens a UDP socket "connected" to a public address (no packet is
/// actually sent - UDP `connect()` only asks the OS to pick a local source
/// address/route) to learn the outbound-interface IP, and ASSUMES a `/24`
/// (255.255.255.0) netmask, which is correct on most flat home/office/lab
/// networks but not on every network. If your subnet is not a `/24`, pass a
/// real `IP_Address`/`IP_Subnet_Mask` into the Network Port some other way
/// (e.g. read it from the OS's own tools, or add a `--netmask` flag) rather
/// than trusting this function blindly in production.
pub fn get_local_ipv4() -> LocalIpv4 {
    let address = std::net::UdpSocket::bind("0.0.0.0:0")
        .and_then(|probe| {
            probe.connect(("8.8.8.8", 80))?;
            match probe.local_addr()?.ip() {
                std::net::IpAddr::V4(v4) => Ok(v4),
                std::net::IpAddr::V6(_) => Ok(Ipv4Addr::LOCALHOST),
            }
        })
        .unwrap_or(Ipv4Addr::LOCALHOST); // no network: loopback keeps the example runnable offline

    if address.is_loopback() {
        return LocalIpv4 {
            address: Ipv4Addr::new(127, 0, 0, 1),
            netmask: Ipv4Addr::new(255, 0, 0, 0),
            broadcast: Ipv4Addr::new(127, 255, 255, 255),
        };
    }

    let netmask = Ipv4Addr::new(255, 255, 255, 0);
    let ip_octets = address.octets();
    let mask_octets = netmask.octets();
    let broadcast_octets = [
        ip_octets[0] | !mask_octets[0],
        ip_octets[1] | !mask_octets[1],
        ip_octets[2] | !mask_octets[2],
        ip_octets[3] | !mask_octets[3],
    ];
    LocalIpv4 {
        address,
        netmask,
        broadcast: Ipv4Addr::from(broadcast_octets),
    }
}

// -----------------------------------------------------------------------------
// The three common callbacks
// -----------------------------------------------------------------------------

/// The stack asks for one queued datagram per call; it never touches the
/// socket itself. Wrapped in `catch_unwind` - see the module doc's "panic
/// safety" note.
extern "C" fn on_receive(
    message: *mut u8,
    max_message_length: u16,
    source_connection_string: *mut u8,
    source_connection_string_length: *mut u8,
    _destination_connection_string: *mut u8,
    destination_connection_string_length: *mut u8,
    max_connection_string_length: u8,
    out_network_port_instance: *mut u32,
) -> u16 {
    let result = catch_unwind(|| {
        // SAFETY: these pointers are supplied by the stack for the duration
        // of this call, per the adapter's documented contract.
        unsafe {
            // The out-params arrive UNINITIALIZED - write every one we do
            // not fill with real data, or the stack reads garbage.
            *source_connection_string_length = 0;
            *destination_connection_string_length = 0;
            if max_connection_string_length < 6 {
                return 0; // cannot even fit an IPv4 connection string
            }
            let datagram = match UDP.lock().unwrap().recv() {
                Some(d) => d,
                None => return 0, // nothing waiting this tick
            };
            let length = datagram.message.len();
            if length > max_message_length as usize {
                eprintln!(
                    "Error: dropping {length}-byte datagram from {}:{} - larger than the stack's {max_message_length}-byte receive buffer.",
                    datagram.from_ip, datagram.from_port
                );
                return 0;
            }
            std::ptr::copy_nonoverlapping(datagram.message.as_ptr(), message, length);
            // 6-byte IPv4 connection string: 4 IP octets, then the port
            // BIG-endian.
            let octets = datagram.from_ip.octets();
            for i in 0..4 {
                *source_connection_string.add(i) = octets[i];
            }
            *source_connection_string.add(4) = (datagram.from_port >> 8) as u8;
            *source_connection_string.add(5) = (datagram.from_port & 0xFF) as u8;
            *source_connection_string_length = 6;
            // Which Network Port object this datagram arrived on.
            *out_network_port_instance = constants::NETWORK_PORT_INSTANCE;
            length as u16
        }
    });
    result.unwrap_or(0)
}

/// Fires from inside `BACnetStack_Tick()` and synchronously from every
/// `BACnetStack_Send*` export. THE RETURN IS A FLAG, NOT A COUNT - the stack
/// only tests it against zero.
extern "C" fn on_send(
    message: *const u8,
    message_length: u16,
    connection_string: *const u8,
    connection_string_length: u8,
    _network_port_instance: u32,
    _broadcast: bool,
) -> u16 {
    let result = catch_unwind(|| {
        if connection_string_length < 6 {
            return 0;
        }
        // SAFETY: the stack guarantees these buffers are valid for the
        // duration of this call.
        let (to_ip, to_port, buffer) = unsafe {
            let octets = std::slice::from_raw_parts(connection_string, 6);
            let to_ip = Ipv4Addr::new(octets[0], octets[1], octets[2], octets[3]);
            let to_port = ((octets[4] as u16) << 8) | (octets[5] as u16);
            let buffer = std::slice::from_raw_parts(message, message_length as usize).to_vec();
            (to_ip, to_port, buffer)
        };
        UDP.lock().unwrap().send(&buffer, to_ip, to_port);
        message_length
    });
    result.unwrap_or(0)
}

/// Unix epoch SECONDS.
extern "C" fn on_get_system_time() -> i64 {
    catch_unwind(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    })
    .unwrap_or(0)
}

/// Register the receive/send/system-time callbacks against the shared
/// `SimpleUdp`. Call once, after the CAS BACnet Stack library is resolvable
/// and before `BACnetStack_AddDevice`.
///
/// Unlike the C++/C#/Python editions, this takes no `network_port_instance`
/// parameter: the callbacks are plain function pointers that read
/// `constants::NETWORK_PORT_INSTANCE` directly (see the module doc).
pub fn register_common_callbacks() {
    bacnet::BACnetStack_RegisterCallbackReceiveMessageForPort(Some(on_receive))
        .expect("BACnetStack_RegisterCallbackReceiveMessageForPort symbol lookup failed");
    bacnet::BACnetStack_RegisterCallbackSendMessageForPort(Some(on_send))
        .expect("BACnetStack_RegisterCallbackSendMessageForPort symbol lookup failed");
    bacnet::BACnetStack_RegisterCallbackGetSystemTime(Some(on_get_system_time))
        .expect("BACnetStack_RegisterCallbackGetSystemTime symbol lookup failed");
}

// -----------------------------------------------------------------------------
// I-Am
// -----------------------------------------------------------------------------

/// Broadcast an unsolicited I-Am announcing this device - every example
/// sends one on start-up. Targets the LOCAL subnet broadcast (the device's
/// own network) rather than the global 255.255.255.255 / network 0xFFFF.
pub fn send_i_am(device_instance: u32, udp_port: u16) -> bool {
    let local = get_local_ipv4();
    let broadcast_octets = local.broadcast.octets();
    let connection_string: [u8; 6] = [
        broadcast_octets[0],
        broadcast_octets[1],
        broadcast_octets[2],
        broadcast_octets[3],
        (udp_port >> 8) as u8,
        (udp_port & 0xFF) as u8,
    ];
    bacnet::BACnetStack_SendIAm(
        device_instance,
        connection_string.as_ptr(),
        6,
        constants::NETWORK_PORT_INSTANCE,
        true, // broadcast
        0,    // destinationNetwork: local network
        std::ptr::null(),
        0,
    )
    .unwrap_or(false)
}

// -----------------------------------------------------------------------------
// CLI helpers
// -----------------------------------------------------------------------------

pub fn print_version(app_name: &str, app_version: &str) {
    println!("{app_name} v{app_version} (common v{COMMON_VERSION})");
    println!(
        "CAS BACnet Stack v{}.{}.{}.{}",
        bacnet::BACnetStack_GetAPIMajorVersion().unwrap_or(0),
        bacnet::BACnetStack_GetAPIMinorVersion().unwrap_or(0),
        bacnet::BACnetStack_GetAPIPatchVersion().unwrap_or(0),
        bacnet::BACnetStack_GetAPIBuildVersion().unwrap_or(0),
    );
}

/// `--port <n>`: an integer 1..65535.
pub fn parse_port(raw: &str) -> Result<u16, String> {
    raw.parse::<u16>()
        .map_err(|_| format!("--port expects an integer 1..65535, got \"{raw}\""))
        .and_then(|v| {
            if v == 0 {
                Err(format!("--port expects an integer 1..65535, got \"{raw}\""))
            } else {
                Ok(v)
            }
        })
}

/// `--deviceID <inst>`: an integer 0..4194302. 4194303 is the BACnet
/// "unconfigured" sentinel - a real device may not use it.
pub fn parse_device_id(raw: &str) -> Result<u32, String> {
    raw.parse::<u32>()
        .map_err(|_| format!("--deviceID expects an integer 0..4194302, got \"{raw}\""))
        .and_then(|v| {
            if v > 4194302 {
                Err(format!(
                    "--deviceID expects an integer 0..4194302, got \"{raw}\""
                ))
            } else {
                Ok(v)
            }
        })
}
