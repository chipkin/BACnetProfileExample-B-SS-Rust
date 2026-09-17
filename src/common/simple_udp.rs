// SPDX-License-Identifier: CC0-1.0
// Public-domain example code (CC0) - see ../../LICENSE.

//! simple_udp.rs
//! =========================================================================
//! A minimal UDP wrapper for the BACnet examples - the Rust edition of the
//! C++ examples' `common/SimpleUDP.{h,cpp}` (and the C#/Python editions'
//! `common/SimpleUDP.cs` / `common/simple_udp.py`), with the same
//! responsibilities:
//!
//!   - own ONE datagram socket bound to the BACnet/IP port,
//!   - let the stack PULL inbound datagrams one at a time (`recv()`) from
//!     its receive callback (the stack never touches the socket - the
//!     application does),
//!   - send outbound datagrams where the stack's send callback points.
//!
//! NO TRANSPORT HELPER EXISTS IN THE RUST ADAPTER DIRECTORY (unlike the
//! other language adapters, which do not ship one either, but this file has
//! no sibling anywhere in `submodules/cas-bacnet-stack/adapters/rust/` to
//! crib from) - this is written from scratch against `std::net::UdpSocket`,
//! matching the shape/responsibilities of the C#/Python siblings above.
//!
//! Poll-based, not queue-based: `recv()` calls the OS socket's `recv_from`
//! directly, non-blocking, once per stack tick. That keeps this whole class
//! single-threaded, matching the other editions' synchronous model - there
//! is no lock anywhere in this file, and there must not be a background
//! thread touching this socket (the CAS BACnet Stack is single-threaded by
//! contract - see AGENTS.md).
//!
//! This struct never sees a BACnet "connection string" - it deals in
//! host-order ip/port pairs. Packing the 6-byte connection string (4 IP
//! octets + 2 port bytes, port BIG-endian) is `cas_example_helper`'s job,
//! exactly as in the other editions.
//! =========================================================================

use std::io::ErrorKind;
use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};

/// One received datagram, handed to the stack's receive callback.
pub struct ReceivedDatagram {
    pub message: Vec<u8>,
    pub from_ip: Ipv4Addr,
    pub from_port: u16,
}

/// An application-owned, non-blocking UDP socket.
pub struct SimpleUdp {
    socket: Option<UdpSocket>,
}

impl SimpleUdp {
    pub const fn new() -> Self {
        SimpleUdp { socket: None }
    }

    /// Bind the socket. Returns `Err` on bind failure (for example: another
    /// BACnet device already owns the port exclusively).
    pub fn setup(&mut self, port: u16) -> std::io::Result<()> {
        let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, port))?;
        socket.set_broadcast(true)?;
        // Non-blocking: recv() must return immediately (None if nothing is
        // waiting) so BACnetStack_Tick() is never delayed by a socket read -
        // this struct is polled once per tick from main.rs's own loop, not
        // from a background thread.
        socket.set_nonblocking(true)?;
        self.socket = Some(socket);
        Ok(())
    }

    /// The next queued inbound datagram, or `None` if none arrived.
    pub fn recv(&self) -> Option<ReceivedDatagram> {
        let socket = self.socket.as_ref()?;
        // 2048 bytes: comfortably larger than the BACnet/IP APDU max (1497).
        let mut buffer = [0u8; 2048];
        match socket.recv_from(&mut buffer) {
            Ok((length, addr)) => match addr {
                std::net::SocketAddr::V4(v4) => Some(ReceivedDatagram {
                    message: buffer[..length].to_vec(),
                    from_ip: *v4.ip(),
                    from_port: v4.port(),
                }),
                std::net::SocketAddr::V6(_) => None, // this example is IPv4-only
            },
            Err(ref e) if e.kind() == ErrorKind::WouldBlock => None, // nothing waiting this tick
            Err(e) => {
                eprintln!("Error: UDP receive failed: {e}");
                None
            }
        }
    }

    /// Send one datagram. Fire-and-forget by design: UDP gives no delivery
    /// guarantee anyway, so a send error is logged, not propagated - same
    /// behaviour as the other editions' `SimpleUDP.send`.
    pub fn send(&self, message: &[u8], to_ip: Ipv4Addr, to_port: u16) {
        if let Some(socket) = &self.socket {
            if let Err(e) = socket.send_to(message, SocketAddrV4::new(to_ip, to_port)) {
                eprintln!("Error: UDP send to {to_ip}:{to_port} failed: {e}");
            }
        }
    }

    /// Close the socket (best-effort; dropping the `UdpSocket` does this).
    pub fn shutdown(&mut self) {
        self.socket = None;
    }
}
