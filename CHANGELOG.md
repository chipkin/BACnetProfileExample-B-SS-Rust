# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.2] - unreleased

### Changed

- **Device renamed from the series' colour placeholder "Rainbow" to "Chipkin
  Example B-SS"** so devices from different examples in the series are
  distinguishable from each other on the same BACnet network - every example
  previously announced the identical Object_Name "Rainbow", which made two
  examples on one subnet indistinguishable by name. Sub-object names are
  unchanged. `docs/colour-table.md` (series root) updated to match.
  APP_VERSION bumped 1.0.1 -> 1.0.2.

## [1.0.1] - 2026-09-22

### Fixed

- `Application_Software_Version` (12) and `Firmware_Revision` (44) were
  hardcoded `"1.0.0"` constants that never tracked reality - the same class
  of bug just found and fixed in the sibling C++ example
  (BACnetProfileExample-B-SCHUB-CPP) by someone reading a real device with
  CAS BACnet Explorer and noticing the reported version didn't match the
  running build. `Application_Software_Version` now reads `APP_VERSION`
  directly (the same constant `--version`'s own banner uses - one source of
  truth, can't drift again). `Firmware_Revision` is now built once at
  start-up, in `main()` right after the native library loads, from the CAS
  BACnet Stack's own `BACnetStack_GetAPIMajorVersion`/`MinorVersion`/
  `PatchVersion`/`BuildVersion` - the same four calls
  `cas_example_helper::print_version` already uses for the start-up banner -
  and stored in `STATE.firmware_revision` (the existing
  `once_cell`-backed `Mutex<DeviceState>` pattern already used for
  start-up-resolved fields like the Network Port's IP addressing).
  `Firmware_Revision` was never meant to be this example's own version; it
  names the platform underneath the app.

## [1.0.0] - 2026-09-16

### Added

- The **first B-SS (Smart Sensor) Rust implementation** in this example
  series, and the **first Rust example in the series overall** - ported from
  [BACnetProfileExample-B-SS-CPP](https://github.com/chipkin/BACnetProfileExample-B-SS-CPP)
  (same device model, same object/property split between "app" and "stack",
  same section layout) with the documentation skeleton carried over from the
  C#/Python editions. The binding mechanism is new to this series in one
  specific way: `libloading`-based lazy symbol resolution plus free-standing
  `extern "C" fn` callbacks reading from a shared `Mutex`-guarded static,
  because Rust's `BACnetStack_RegisterCallbackGetProperty*` bindings take
  plain function pointers, not closures - see `src/common/CHANGELOG.md` for
  the full list of what that changes.
- The complete B-SS example application (`src/main.rs`): device 389001
  ("Rainbow"), the series' three read-only input objects (Analog Input
  "Bronze", Binary Input "Emerald", Multi-State Input "Hot Pink") plus the
  required Network Port ("Vermilion"), DS-RP-B (ReadProperty), DM-DDB-B /
  DM-DOB-B (Who-Is/I-Am, Who-Has/I-Have), unsolicited I-Am on start-up,
  series-standard CLI (`--help`/`--version`/`--deviceID`/`--port`, hand-rolled
  - no `clap` dependency) and interactive commands (`h`/`q`/`up`/`down`, via a
  line-buffered background-thread reader - a documented simplification from
  the other editions' raw single-keypress polling, since Rust's standard
  library has no portable non-blocking single-keypress read; see
  TUTORIAL.md). There are no `Set*` callbacks and no
  `SERVICES_SUPPORTED_WRITE_PROPERTY` enable anywhere - this device is
  read-only end to end, matching the B-SS profile boundary.
- Every `Get*Property` callback and every transport callback wrapped in
  `std::panic::catch_unwind`, returning the "no opinion"/"nothing sent" value
  on a caught panic rather than letting it unwind across the `extern "C"`
  boundary into the native stack (undefined behavior otherwise). This
  example's own build is, as far as this project is aware, **the first
  automated CI compilation gate the Rust adapter has ever had** - see
  `src/cas_bacnet_stack/README.md` and AGENTS.md; treat any future change to
  the vendored adapter files as unverified again until this (or another) CI
  re-checks it.
- Vendored Rust `src/common/` v1.0.0: `simple_udp.rs` (written from scratch -
  no transport helper exists in the stack's `adapters/rust/` to vendor),
  `cas_example_helper.rs` (transport callbacks + the 6-byte IPv4 connection
  string, I-Am, local-IP discovery via a UDP-connect probe with an assumed
  `/24` netmask, CLI helpers), `constants.rs` (the **complete** set of BACnet
  enumeration values this example needs - the Rust adapter defines none of
  its own, unlike the C#/Python adapters), `device_state.rs` (new: the
  `once_cell::sync::Lazy<Mutex<DeviceState>>` static every callback reads
  from - no equivalent file in the other editions, made necessary by Rust's
  function-pointer-only callback registration). See `src/common/CHANGELOG.md`
  for the full list of what targeting the `submodules/cas-bacnet-stack` `6.x`
  branch's Rust adapter means.
- `src/cas_bacnet_stack/` - the vendored Rust FFI adapter
  (`cas_bacnet_stack_adapters.rs`, 14,000+ generated lines;
  `property_buffer_helper.rs`, vendored for parity but unused by this
  read-only example), copied (not referenced) from
  `submodules/cas-bacnet-stack/adapters/rust/`, with one deliberate
  hand-edit: `load_library()` now resolves
  `CASBACnetStack_x64_Release{.dll,.so,.dylib}` next to the running
  executable instead of the generated default's `./bin/CASBACnetStack_x64_Debug`
  (a Debug-named DLL under a working-directory-relative path that does not
  match this example's Release-only build/ship convention) - see
  `src/cas_bacnet_stack/README.md`.
- Repository scaffold: CAS BACnet Stack submodule
  (`submodules/cas-bacnet-stack`, tracking `6.x`, pinned at commit
  `ec60c71801aad076a46c0ec783c5566a6cd0f3a0` - the same commit the C#/Python
  siblings are pinned at), CC0-1.0 licence, README, TUTORIAL, `docs/PICS.md` +
  `docs/objects.json` (identical content to the C++/C#/Python editions - same
  device model), AGENTS.md, `.github/workflows/release.yml` (native-library
  build + `cargo build --release` + smoke test + release-on-tag, adapted from
  the C# edition's workflow).

Verified locally: `cargo build --release` succeeds with **0 warnings, 0
errors** (one `#![allow(dead_code)]` in the vendored adapter file, documented
in place and in `src/cas_bacnet_stack/README.md` - the generated file's own
231 exports are a complete binding surface, of which this minimal example
calls roughly a dozen). The built binary, run with `--port 47834` next to the
native `CASBACnetStack_x64_Release.dll`, prints the version banner and the
"ready" line and stays up as a live BACnet/IP device until stopped.

Not yet exercised in this change: a live BACnet client session (Who-Is →
I-Am; ReadProperty of every required property; `State_Text[4]` erroring
`invalid-array-index`; WriteProperty rejection) - the local smoke test
confirms the device starts, binds its socket, and stays running, but this
change did not run a full client-side conformance pass. Follow
[TUTORIAL.md "Reviewing your device"](TUTORIAL.md#reviewing-your-device)
before relying on this as a certified-equivalent reference.
