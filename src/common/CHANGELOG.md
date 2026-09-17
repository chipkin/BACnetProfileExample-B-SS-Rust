# Changelog - `common/` (Rust edition)

All notable changes to the vendored Rust `common/` helpers. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [1.0.0] - 2026-09-16

### Added

- `simple_udp.rs` - application-owned, non-blocking UDP socket
  (`std::net::UdpSocket`, `set_nonblocking(true)`). Written from scratch: no
  transport helper exists anywhere in
  `submodules/cas-bacnet-stack/adapters/rust/` to vendor, unlike this
  repository's other files. Poll-based (`recv()` calls `recv_from` directly
  once per stack tick), matching the C++/C#/Python editions' single-threaded
  model - no lock, no background thread.
- `cas_example_helper.rs` - `register_common_callbacks()` (receive/send/
  system-time callbacks, as free-standing `extern "C" fn` items - see below;
  the 6-byte IPv4 connection string, 4 octets + big-endian port, packed here
  once), `send_i_am()` targeting the local subnet broadcast, `get_local_ipv4()`
  (a UDP-connect probe + assumed `/24` netmask, matching the Python edition's
  documented deviation - Rust's standard library has no portable interface
  enumeration either), and CLI helpers (`--help`/`--version`/`--deviceID`/
  `--port`, hand-rolled - no `clap` dependency).
- `constants.rs` - the **complete** set of BACnet enumeration values this
  example needs. Unlike the C#/Python editions (which reuse most values
  directly from their vendored adapter's own enumeration block), the Rust
  adapter defines **no** `OBJECT_TYPE_*`/`PROPERTY_IDENTIFIER_*`/
  `SERVICES_SUPPORTED_*` constants at all - it is a pure FFI binding surface
  (function signatures only, generated from `source/CASBACnetStackDLL.h`).
  Every value here is therefore newly written, not ported from an existing
  adapter block, though every value matches the BACnet standard and the
  C++/C#/Python editions' equivalent files 1:1.
- `device_state.rs` - new file, no equivalent in the C++/C#/Python editions.
  A `once_cell::sync::Lazy<Mutex<DeviceState>>` static holding every piece of
  mutable device state a `Get*Property` callback needs to read. Necessary
  because `BACnetStack_RegisterCallbackGetProperty*` takes
  `Option<extern "C" fn(...)>` - a plain, non-capturing function pointer -
  so a callback cannot close over a `main()`-local the way the C++/C#/Python
  editions' lambdas/delegates/closures do. See the file's module doc for the
  full explanation; this is the single biggest structural difference this
  edition has from its siblings.

### First Rust `common/` in this series - what makes it different from the C++/C#/Python editions

This is the first Rust `common/` in the BACnet profile example series, so
there is no prior Rust pin to diff against. The systematic differences versus
the C++/C#/Python editions (all targeting the same
`submodules/cas-bacnet-stack` `6.x` branch and the same callback API -
trailing `errorCode` out-param, folded `AddNetworkPortObject()`, `*ForPort`
transport callbacks keyed by Network Port instance - none of that is new
here):

- **Explicit `libloading` resolution, like C++.** Unlike the C# edition
  (implicit P/Invoke resolution on first call), the Rust adapter's
  `once_cell::sync::Lazy<libloading::Library>` resolves the native library
  lazily on the first `bacnet::` call - functionally similar to the C++
  edition's explicit `LoadBACnetFunctions()` step, except the "load" is
  implicit in Rust's `Lazy` rather than a function `main.rs` calls itself.
  `main.rs` treats that first call (inside `print_version()`) as the load
  check, wrapped in `std::panic::catch_unwind` because the vendored
  `load_library()` panics (rather than returning a `Result`) on failure -
  see `cas_bacnet_stack/README.md`.
- **No enumeration constants in the adapter at all** (see `constants.rs`
  above) - the biggest practical difference from the C#/Python editions'
  equivalent files.
- **Plain function pointers, not closures or delegates.** The C++ edition's
  lambdas can capture `this`/locals; C#'s delegates and Python's
  `ctypes.CFUNCTYPE` wrappers can close over local state too. Rust's
  `extern "C" fn` items cannot capture anything - hence `device_state.rs`
  and the module-level `Lazy<Mutex<SimpleUdp>>` in this file.
- **Explicit panic-safety wrapping.** Every callback here and in `main.rs`
  wraps its body in `std::panic::catch_unwind` - an unguarded panic unwinding
  across an `extern "C" fn` into the native stack is undefined behavior, and
  this stack's Rust adapter has no CI-verified track record (see
  `cas_bacnet_stack/README.md`), so this matters more here than in the other
  editions.
- **Raw pointers, matching the native ABI 1:1** - same as the C# edition's
  `byte*`/`uint*`/`float*` delegates (vs. the Node adapter's marshaled
  `Buffer`s, not present in this series yet). Every pointer-touching function
  in this file and `main.rs` is `unsafe` at the point of use.
