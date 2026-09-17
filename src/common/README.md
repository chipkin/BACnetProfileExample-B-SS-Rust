# `common/` - shared example plumbing (Rust edition)

The helpers every Rust example in the BACnet profile example series will
share. **Vendored**: this directory is a *copy* in each example repo, not a
crate published to crates.io - change it in one repo and you must sweep the
same change to every sibling and bump `COMMON_VERSION` (in
`cas_example_helper.rs`) + add a `CHANGELOG.md` entry (this file's sibling,
one directory up). This is the **first Rust example** in the series, so there
is no sibling to sweep to yet - the next Rust example in the series starts by
copying this folder.

## Versioning

`COMMON_VERSION` in `cas_example_helper.rs`, changelog in
`src/common/CHANGELOG.md`. The Rust `common/` versions independently of the
C++/C#/Python `common/` directories (each at its own version) - same rules,
separate lineage. This copy starts at **1.0.0**: the first Rust `common/`,
built against the `submodules/cas-bacnet-stack` `6.x` branch's current
adapter/callback API.

## What's here

| File | What it is |
|---|---|
| `simple_udp.rs` | The UDP socket the application owns: bind, poll for inbound datagrams (non-blocking `recv_from`, no background thread), send. The stack never touches the socket - it pulls datagrams through the receive callback. Written from scratch against `std::net::UdpSocket` - unlike the vendored adapter directories for other languages, `submodules/cas-bacnet-stack/adapters/rust/` ships no transport helper to crib from. |
| `cas_example_helper.rs` | `register_common_callbacks()` (receive/send/system-time + the 6-byte IPv4 connection string, port big-endian, written here ONCE), `send_i_am()`, `get_local_ipv4()`, and CLI helpers. Also owns the one shared `SimpleUdp` instance, because the transport callbacks are plain `extern "C" fn` items that cannot capture it - see this file's own module doc. |
| `constants.rs` | The **complete** set of BACnet enumeration values this example needs (`OBJECT_TYPE_*`, `PROPERTY_IDENTIFIER_*`, `SERVICES_SUPPORTED_*`, ...) - unlike the C#/Python editions' equivalent file, which only adds the handful their adapters don't already define, `cas_bacnet_stack_adapters.rs` (the vendored Rust adapter) defines **no** enumeration constants at all, so every value lives here. See the file's header comment. |
| `device_state.rs` | The `once_cell::sync::Lazy<Mutex<DeviceState>>` static every `Get*Property` callback in `main.rs` reads from - the single most important file in this example for understanding why the Rust edition is shaped the way it is. See its module doc. |

## How main.rs uses it

```rust
common::cas_example_helper::setup_udp(port)?;
common::cas_example_helper::register_common_callbacks(); // before AddDevice
// ... AddDevice, objects, services ...
common::cas_example_helper::send_i_am(device_instance, port); // announce on start-up
while running.load(Ordering::SeqCst) {
    bacnet::BACnetStack_Tick();
    std::thread::sleep(Duration::from_millis(1));
}
```

Unlike the C++/C#/Python `common/`, there is no `RestartKind`/
`RequestRestart`/`RestartDue` deferred-restart pattern here: B-SS does not
implement DM-RD-B (ReinitializeDevice), so this copy of `common/` does not
carry code for a capability no example using it needs yet.

## Why `extern "C" fn`, not closures

`BACnetStack_RegisterCallback*` takes `Option<extern "C" fn(...)>` - a plain,
non-capturing function pointer. `register_common_callbacks()`'s transport
callbacks (`on_receive`, `on_send`, `on_get_system_time`) are therefore
free-standing items, not closures, and reach the shared `SimpleUdp` through a
module-level `Lazy<Mutex<SimpleUdp>>` static instead of capturing it - the
same pattern `device_state.rs` documents for the property callbacks in
`main.rs`. Every one of them wraps its body in `std::panic::catch_unwind`
too - see AGENTS.md "panic safety across the FFI boundary".
