# BACnet B-SS (Smart Sensor) - Rust example

A minimal, copy-paste-friendly example showing how to implement the BACnet
**B-SS (Smart Sensor)** device profile in Rust, using the
[CAS BACnet Stack](https://store.chipkin.com/services/stacks/bacnet-stack). It
listens on **BACnet/IP (UDP 47808)**, answers **ReadProperty**, and is
discoverable via **Who-Is / I-Am**.

This is the **first Rust example** in the BACnet profile example series.
There is no prior Rust example repo to copy structurally - this one was built
from the C++ edition's device model and the C#/Python editions' documentation
skeleton, with a binding mechanism new to the series in one specific way:
Rust's `libloading` resolves the native library's symbols lazily (similar in
spirit to the C++ edition's explicit `LoadBACnetFunctions()`, unlike C#'s
implicit P/Invoke resolution), and every property-get callback is a
free-standing `extern "C" fn` reading from a shared `Mutex`-guarded static,
because Rust callback registration takes plain function pointers, not
closures - see [TUTORIAL.md](TUTORIAL.md#why-callbacks-read-from-device_statestate)
for why that shapes this whole example.

- **[TUTORIAL.md](TUTORIAL.md)** - how to extend this example and how to review
  it for conformance. Read it when you start turning this into your own device.
- **[docs/PICS.md](docs/PICS.md)** - the Protocol Implementation Conformance
  Statement: every object, every property, and who answers it.

> **Versions:** this document describes **example v1.0.0**, built and verified
> against the CAS BACnet Stack submodule pinned at `6.x`, at
> **Protocol_Revision 24**, with the vendored Rust `common/` helper at
> **v1.0.0**. Running the example prints the app and API versions - if what it
> prints disagrees with this line, trust the program and check `CHANGELOG.md`.

## What is the B-SS (Smart Sensor) profile?

**B-SS (BACnet Smart Sensor)**, defined in Annex L of ANSI/ASHRAE 135, is the
simplest BACnet device profile - the standard describes it as *"a simple
sensing device with very limited resources."* It is meant for inexpensive,
fixed-function sensors (temperature, humidity, occupancy, a contact, ...) that
mostly just need to **report what they measure** when asked.

A B-SS device answers **ReadProperty** and is discoverable. It does not have
to support **WriteProperty** (a smart sensor is read-only), **alarming /
event reporting**, **scheduling**, or **trending**, and this example
implements none of them on purpose.

**But it is still a full BACnet device.** Even the simplest profile must
present the standard object model - a **Device** object, a **Network Port**
object (every device needs one), and its sensor objects - and each object
must expose all of its **required properties**. The CAS BACnet Stack
generates most of those automatically (Object_Identifier, Object_Type,
Status_Flags, Object_List, Protocol_*, ...); this example supplies the
handful that are application-specific. The result is conformant for
**Protocol_Revision 24**. [docs/PICS.md](docs/PICS.md) lists every property
and who answers it.

## The device this example creates

```
Device 389001  "Chipkin Example B-SS"   (Vendor 389 - Chipkin Automation Systems)
    │
    ├── Analog Input 1       "Bronze"      Present_Value  21.5     (REAL, degrees Celsius; read-only)
    ├── Binary Input 1       "Emerald"     Present_Value  inactive (read-only)
    ├── Multi-State Input 1  "Hot Pink"    Present_Value  1        (state 1..3; read-only)
    └── Network Port 1       "Vermilion"   the BACnet/IP port     (required on every device)
```

## What this example supports

The example implements exactly the capabilities below - and nothing more,
which is the point of a profile example.

### BIBBs (BACnet Interoperability Building Blocks)

| BIBB | Description | Supported |
|------|-------------|:---------:|
| DS-RP-B | Data Sharing - ReadProperty - B | ✅ |
| DM-DDB-B | Device Management - Dynamic Device Binding - B | ✅ |
| DM-DOB-B | Device Management - Dynamic Object Binding - B | ✅ |

### Services (executed / B-side)

| Service | Notes |
|---------|-------|
| ReadProperty | Responds to property reads (DS-RP-B). |
| Who-Is / I-Am | Answers Who-Is with I-Am, and broadcasts an I-Am on start-up (DM-DDB-B). |
| Who-Has / I-Have | Answers Who-Has with I-Have (DM-DOB-B). |

### Object types

| Object type | Instance | Name |
|-------------|:--------:|------|
| Device | 389001 | Chipkin Example B-SS |
| Analog Input | 1 | Bronze |
| Binary Input | 1 | Emerald |
| Multi-state Input | 1 | Hot Pink |
| Network Port | 1 | Vermilion |

Every required property of every object, and who answers it, is in
[docs/PICS.md](docs/PICS.md).

## Requires the CAS BACnet Stack (licensed product)

This example **builds against the CAS BACnet Stack, which is a commercial
Chipkin product** - it is not free or open source, and there is no
public/trial build. The stack is referenced here as the **private** git
submodule `submodules/cas-bacnet-stack` (tracking the `6.x` branch). Like the
C#/Python editions (and unlike the C++ edition's default "compile the stack's
own source straight into the executable" mode), this Rust edition links
against a **separately built native shared library**
(`CASBACnetStack_x64_Release.dll` on Windows / `libCASBACnetStack_x64_Release.so`
on Linux) - loaded at run time via the vendored `libloading`-based Rust
adapter, not compiled in - see
[Build the native CAS BACnet Stack library](#build-the-native-cas-bacnet-stack-library)
below. **To get the CAS BACnet Stack (and access to build this example),
contact Chipkin:** <https://store.chipkin.com/services/stacks/bacnet-stack> or
sales@chipkin.com.

## What's in this repository

This is a **self-contained** project. It ships:

- `src/main.rs` - the example device.
- `src/common/` - the shared Rust helper (`simple_udp.rs`, `cas_example_helper.rs`,
  `constants.rs`, `device_state.rs`) vendored in - see `src/common/README.md`.
- `src/cas_bacnet_stack/` - the vendored Rust FFI adapter, copied from the
  stack submodule's `adapters/rust/` with one deliberate hand-edit
  (`load_library()`) - see `src/cas_bacnet_stack/README.md`.
- `Cargo.toml` - the build (edition 2021), the same on Windows and Linux.
- `docs/PICS.md` - the conformance statement.
- `docs/objects.json` - the input to the objects-and-properties generator.
- `submodules/cas-bacnet-stack/` - the **CAS BACnet Stack as a git submodule**
  (private; requires a license - see above). Its `source/` and MSVC/CMake
  build produce the separate native shared library this example loads at run
  time; its `adapters/rust/` is what `src/cas_bacnet_stack/` is vendored from.

## Prerequisites

- **Rust** (`cargo --version` / `rustc --version`) - edition 2021 or later
  toolchain (stable channel).
- Git (to fetch the stack submodule).
- **To build the native library** (see below): on Windows, Visual Studio 2022
  Build Tools (MSBuild) targeting the stack's `projects/msvs/BuildCASBACnetStack.sln`;
  on Linux, a C++17 toolchain (g++/clang++).

## Build

```bash
git clone --recursive https://github.com/chipkin/BACnetProfileExample-B-SS-Rust.git
cd BACnetProfileExample-B-SS-Rust

cargo build --release
```

Already cloned without `--recursive`? Run `git submodule update --init --recursive`
first - the native-library build (below) needs the stack submodule's
`source/`.

`cargo build` compiles this example and the vendored Rust adapter (a `mod`
inside this crate, not a separate published crate) - it does **not** build
the native CAS BACnet Stack library the adapter dlopen's at run time. That is
a separate step (below), because it is a native C++ build, not a Cargo one.

### Build the native CAS BACnet Stack library

The vendored adapter's `load_library()` resolves
`CASBACnetStack_x64_Release.dll` (Windows) / `libCASBACnetStack_x64_Release.so`
(Linux) **next to the running executable** at run time (see
`src/cas_bacnet_stack/README.md` for exactly how, and why that differs from
the file this was vendored from) - there is no bundled copy and no crates.io
package for the native library. Build it once from the stack submodule, then
copy it next to this project's build output.

**Windows** - the stack's MSVC solution has a `ReleaseDll|x64` configuration
that builds exactly this DLL, under exactly this name:

```powershell
& "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\MSBuild\Current\Bin\MSBuild.exe" `
  submodules\cas-bacnet-stack\projects\msvs\BuildCASBACnetStack.sln `
  /p:Configuration=ReleaseDll /p:Platform=x64

copy submodules\cas-bacnet-stack\bin\CASBACnetStack_x64_Release.dll target\release\
```

(Adjust the MSBuild path to your Visual Studio / Build Tools install; `vswhere`
or the "Developer Command Prompt for VS 2022" both resolve it for you if you
have Visual Studio Installer set up.)

**Linux** - the stack does not currently ship a ready-made shared-library
CMake target. Build one directly from the stack's own sources with g++:

```bash
g++ -shared -fPIC -std=c++17 -O2 \
    -I submodules/cas-bacnet-stack/source \
    submodules/cas-bacnet-stack/source/*.cpp \
    -o target/release/libCASBACnetStack_x64_Release.so
```

Either way, **the DLL/.so must end up next to the compiled executable**
(`target/release/` by default) - that is where `load_library()` looks first.

### Why only a Release build

The vendored adapter's `load_library()` resolves
`CASBACnetStack_x64_Release{.dll,.so,.dylib}` - the generated file this was
copied from instead named a Debug-configuration DLL by default (see
`src/cas_bacnet_stack/README.md` for the full explanation of that edit). The
stack's MSVC solution only has a `ReleaseDll` configuration (see above) - so
build and run this example against that library. `cargo build --release` also
matches the CI workflow and the smoke test below.

### Why `property_buffer_helper.rs` is here but unused

`src/cas_bacnet_stack/` includes both of the adapter's files for parity with
its expected set (see `src/cas_bacnet_stack/README.md`), but
`property_buffer_helper.rs` packs buffer-shaped `Send*` calls
(ReadProperty/WriteProperty/CreateObject **as a client**) that this read-only
B-SS example never makes - a Smart Sensor only ever *answers* ReadProperty,
it never *initiates* one. It compiles in unused (`#[allow(dead_code)]` on its
`mod` declaration); that is a documented, deliberate choice, not an
oversight.

## Run

```bash
cargo run --release -- --port 47821 --deviceID 12345
# or, after `cargo build --release`, run the built executable directly:
./target/release/bacnet-profile-example-b-ss-rust --port 47821
```

Expected output:

```
BACnet B-SS (Smart Sensor) Example - Rust v1.0.0 (common v1.0.0)
CAS BACnet Stack v6.0.21.0
FYI: listening on <your-ip>:47808 (broadcast <your-subnet-broadcast>)
FYI: Device 389001 ("Chipkin Example B-SS") ready. Vendor ID 389. Type 'h' + Enter for help.
```

The device listens on UDP **47808** (BACnet/IP) by default. Allow that port
through your firewall. To use a different port, pass `--port`.

### Command-line options

| Option | Default | Meaning |
|--------|---------|---------|
| `--port <n>` | `47808` | UDP port to listen on (BACnet/IP). |
| `--deviceID <n>` | `389001` | The device's BACnet instance number (BACnet requires this to be configurable). |
| `--help`, `-h` | - | Show usage and exit. |
| `--version` | - | Print the example and stack versions, then exit. |

Hand-rolled parsing (`src/main.rs`'s `parse_args()`) - no `clap` dependency;
see AGENTS.md for why.

### Interactive commands

While the example runs, type a command and press **Enter** (this edition is
line-buffered, not raw single-keypress - see
[TUTORIAL.md](TUTORIAL.md#interactive-commands-are-a-simplification) for why):

| Command | Action |
|---------|--------|
| `h` | Show the version information and this command list. |
| `q` | Quit. |
| `up` | Increase Analog Input 1 (`Bronze`) by 1.1. |
| `down` | Decrease Analog Input 1 (`Bronze`) by 1.1. |

The up/down commands change the live `Present_Value` of the analog input, so
a client re-reading it sees the new value. Ctrl+C also shuts the device down
cleanly (an interactive console is not required for that).

## Verify

Use a BACnet client such as the
[**CAS BACnet Explorer**](https://store.chipkin.com/products/tools/cas-bacnet-explorer):

1. **Discover** - send a **Who-Is**. The device replies with **I-Am** from
   instance **389001** (vendor **389**). It also broadcasts an I-Am at
   start-up.
2. **Browse the object model** - the device shows five objects: the Device
   (`Chipkin Example B-SS`), the three sensors, and the Network Port (`Vermilion`). Reading
   the Device's `Object_List` returns all five.
3. **Read the Device** - ReadProperty `389001` -> `Object_Name` returns
   `"Chipkin Example B-SS"`; `Protocol_Revision` returns `24`; `Description` returns the
   profile description string.
4. **Read a sensor** - ReadProperty Analog Input `1` -> `Present_Value`
   returns `21.5`; `Units` returns `degrees-Celsius`; `Out_Of_Service` returns
   `false`; `Object_Name` returns `"Bronze"`. Repeat for Binary Input `1`
   (`"Emerald"`, has `Polarity`) and Multi-State Input `1` (`"Hot Pink"`, has
   `Number_Of_States` = 3). Every required property of every object is
   readable.
5. **Confirm the profile boundary** - a **WriteProperty** to any object is
   rejected. That is correct: a B-SS Smart Sensor is read-only.

For a property-by-property review against the conformance statement, see
[TUTORIAL.md](TUTORIAL.md).

## How the Rust binding differs from the C++/C#/Python editions

If you have read a sibling example, the object model and callback shapes are
identical - this section is only about what is mechanically different
because Rust talks to the stack through `libloading` and plain function
pointers. See `src/common/CHANGELOG.md` for the full list; the highlights:

- **Lazy `libloading` resolution, not `LoadBACnetFunctions()` or implicit
  P/Invoke.** The vendored adapter's `once_cell::sync::Lazy<libloading::Library>`
  resolves the native library on the first `bacnet::` call - `main.rs` treats
  that first call (inside `print_version()`) as the load check, wrapped in
  `std::panic::catch_unwind` because a failed load panics rather than
  returning a `Result` (see `src/cas_bacnet_stack/README.md`).
- **No enumeration constants in the adapter.** Unlike the C#/Python editions
  (which reuse most BACnet enumeration values directly from their adapter),
  `cas_bacnet_stack_adapters.rs` is a pure FFI binding surface with zero
  `OBJECT_TYPE_*`/`PROPERTY_IDENTIFIER_*` constants - every one is defined in
  `src/common/constants.rs` instead.
- **Plain function pointers, not closures.** `BACnetStack_RegisterCallbackGetProperty*`
  takes `Option<extern "C" fn(...)>`. Every callback is a free-standing item
  reading from `common::device_state::STATE`, a
  `once_cell::sync::Lazy<Mutex<DeviceState>>` - see
  [TUTORIAL.md](TUTORIAL.md#why-callbacks-read-from-device_statestate).
- **Explicit panic-safety wrapping.** Every callback wraps its body in
  `std::panic::catch_unwind` - an unguarded panic unwinding across an
  `extern "C" fn` into the native stack is undefined behavior, and this
  stack's Rust adapter has no CI-verified track record beyond a single manual
  check (see `src/cas_bacnet_stack/README.md`).
- **Raw pointers, matching the native ABI 1:1** - same as the C# edition's
  raw-pointer delegates. Every pointer-touching function is `unsafe` at the
  point of use.

## The BACnet profile example series

<!-- PROFILE-TABLE:BEGIN (generated from cas-bacnet-stack-examples/docs/profile-table.md - do not edit here) -->
The CAS BACnet Stack supports every standardized device profile in ASHRAE 135-2024 Annex L, and there is one example repository per profile. Pick the profile your device claims, then the language you build in. "Ask" means the example hasn't been built yet for that language - [contact Chipkin](https://store.chipkin.com/contact-us) if you need one.

### Controllers (Annex L.4)

| Profile | C++ | Node.js | C# | Rust | Python | Go |
|---|---|---|---|---|---|---|
| **B-SS** Smart Sensor | [B-SS-CPP](https://github.com/chipkin/BACnetProfileExample-B-SS-CPP) | [B-SS-Node](https://github.com/chipkin/BACnetProfileExample-B-SS-Node) | [B-SS-CS](https://github.com/chipkin/BACnetProfileExample-B-SS-CS) | [B-SS-Rust](https://github.com/chipkin/BACnetProfileExample-B-SS-Rust) | [B-SS-Python](https://github.com/chipkin/BACnetProfileExample-B-SS-Python) | [B-SS-Go](https://github.com/chipkin/BACnetProfileExample-B-SS-Go) |
| **B-SA** Smart Actuator | [B-SA-CPP](https://github.com/chipkin/BACnetProfileExample-B-SA-CPP) | Ask | Ask | Ask | Ask | Ask |
| **B-ASC** Application Specific Controller | [B-ASC-CPP](https://github.com/chipkin/BACnetProfileExample-B-ASC-CPP) | [B-ASC-Node](https://github.com/chipkin/BACnetProfileExample-B-ASC-Node) | Ask | Ask | Ask | Ask |
| **B-AAC** Advanced Application Controller | [B-AAC-CPP](https://github.com/chipkin/BACnetProfileExample-B-AAC-CPP) | Ask | Ask | Ask | Ask | Ask |
| **B-BC** Building Controller | [B-BC-CPP](https://github.com/chipkin/BACnetProfileExample-B-BC-CPP) | Ask | Ask | Ask | Ask | Ask |

### Life safety controllers (Annex L.5)

| Profile | C++ | Node.js | C# | Rust | Python | Go |
|---|---|---|---|---|---|---|
| **B-LSC** Life Safety Controller | [B-LSC-CPP](https://github.com/chipkin/BACnetProfileExample-B-LSC-CPP) 🚧 | Ask | Ask | Ask | Ask | Ask |
| **B-ALSC** Advanced Life Safety Controller | [B-ALSC-CPP](https://github.com/chipkin/BACnetProfileExample-B-ALSC-CPP) | Ask | Ask | Ask | Ask | Ask |

### Access control controllers (Annex L.6)

| Profile | C++ | Node.js | C# | Rust | Python | Go |
|---|---|---|---|---|---|---|
| **B-ACC** Access Control Controller | [B-ACC-CPP](https://github.com/chipkin/BACnetProfileExample-B-ACC-CPP) | Ask | Ask | Ask | Ask | Ask |
| **B-AACC** Advanced Access Control Controller | [B-AACC-CPP](https://github.com/chipkin/BACnetProfileExample-B-AACC-CPP) | Ask | Ask | Ask | Ask | Ask |

### Lighting controllers (Annex L.11)

| Profile | C++ | Node.js | C# | Rust | Python | Go |
|---|---|---|---|---|---|---|
| **B-LD** Lighting Device | [B-LD-CPP](https://github.com/chipkin/BACnetProfileExample-B-LD-CPP) | Ask | Ask | Ask | Ask | Ask |
| **B-LS** Lighting Supervisor | [B-LS-CPP](https://github.com/chipkin/BACnetProfileExample-B-LS-CPP) | Ask | Ask | Ask | Ask | Ask |

### Elevator controllers (Annex L.13)

| Profile | C++ | Node.js | C# | Rust | Python | Go |
|---|---|---|---|---|---|---|
| **B-EM** Elevator Monitor | [B-EM-CPP](https://github.com/chipkin/BACnetProfileExample-B-EM-CPP) | Ask | Ask | Ask | Ask | Ask |
| **B-EC** Elevator Controller | [B-EC-CPP](https://github.com/chipkin/BACnetProfileExample-B-EC-CPP) | Ask | Ask | Ask | Ask | Ask |
| **B-AEC** Advanced Elevator Controller | [B-AEC-CPP](https://github.com/chipkin/BACnetProfileExample-B-AEC-CPP) | Ask | Ask | Ask | Ask | Ask |

### Authentication and authorization (Annex L.14)

| Profile | C++ | Node.js | C# | Rust | Python | Go |
|---|---|---|---|---|---|---|
| **B-AS** Authorization Server | [B-AS-CPP](https://github.com/chipkin/BACnetProfileExample-B-AS-CPP) | Ask | Ask | Ask | Ask | Ask |

### Miscellaneous (Annex L.7, combinable with any one family)

| Profile | C++ | Node.js | C# | Rust | Python | Go |
|---|---|---|---|---|---|---|
| **B-BBMD** Broadcast Management Device | [B-BBMD-CPP](https://github.com/chipkin/BACnetProfileExample-B-BBMD-CPP) | Ask | Ask | Ask | Ask | Ask |
| **B-ACDC** Access Control Door Controller | [B-ACDC-CPP](https://github.com/chipkin/BACnetProfileExample-B-ACDC-CPP) | Ask | Ask | Ask | Ask | Ask |
| **B-ACCR** Access Control Credential Reader | [B-ACCR-CPP](https://github.com/chipkin/BACnetProfileExample-B-ACCR-CPP) | Ask | Ask | Ask | Ask | Ask |
| **B-RTR** Router | [B-RTR-CPP](https://github.com/chipkin/BACnetProfileExample-B-RTR-CPP) | Ask | Ask | Ask | Ask | Ask |
| **B-GW** Gateway | [B-GW-CPP](https://github.com/chipkin/BACnetProfileExample-B-GW-CPP) | Ask | Ask | Ask | Ask | Ask |
| **B-DAP** Device Address Proxy | [B-DAP-CPP](https://github.com/chipkin/BACnetProfileExample-B-DAP-CPP) | Ask | Ask | Ask | Ask | Ask |
| **B-SCHUB** BACnet/SC Hub | [B-SCHUB-CPP](https://github.com/chipkin/BACnetProfileExample-B-SCHUB-CPP) | Ask | Ask | Ask | Ask | Ask |
| **B-GENERAL** General device (Annex L.8) | *(satisfied by every example above)* | — | — | — | — | — |

### Operator interfaces and workstations (Annex L.1–L.3, L.9–L.10, L.12)

Client-side profiles.

| Profile | C++ | Node.js | C# | Rust | Python | Go |
|---|---|---|---|---|---|---|
| **B-OD** Operator Display | [B-OD-CPP](https://github.com/chipkin/BACnetProfileExample-B-OD-CPP) | Ask | Ask | Ask | Ask | Ask |
| **B-OWS** Operator Workstation | planned | — | — | — | — | — |
| **B-AWS** Advanced Operator Workstation | planned | — | — | — | — | — |
| **B-XAWS** Extended Advanced Operator Workstation | planned | — | — | — | — | — |
| **B-LSAP** Life Safety Annunciator Panel | planned | — | — | — | — | — |
| **B-LSWS** Life Safety Workstation | planned | — | — | — | — | — |
| **B-ALSWS** Advanced Life Safety Workstation | planned | — | — | — | — | — |
| **B-ACSD** Access Control Security Display | planned | — | — | — | — | — |
| **B-ACWS** Access Control Workstation | planned | — | — | — | — | — |
| **B-AACWS** Advanced Access Control Workstation | planned | — | — | — | — | — |
| **B-LOD** Lighting Operator Display | planned | — | — | — | — | — |
| **B-ALWS** Advanced Lighting Workstation | planned | — | — | — | — | — |
| **B-LCS** Lighting Control Station | planned | — | — | — | — | — |
| **B-ALCS** Advanced Lighting Control Station | planned | — | — | — | — | — |
| **B-ED** Elevator Display | planned | — | — | — | — | — |
| **B-EWS** Elevator Workstation | planned | — | — | — | — | — |
| **B-AEWS** Advanced Elevator Workstation | planned | — | — | — | — | — |

🚧 = in progress. "Ask" = not yet built for that language; contact Chipkin if you need it. Profile definitions: ANSI/ASHRAE 135-2024 Annex L. BIBB definitions: Annex K. Get the stack: <https://store.chipkin.com/services/stacks/bacnet-stack>.
<!-- PROFILE-TABLE:END -->

## References

- **ANSI/ASHRAE Standard 135** (BACnet) - the protocol standard. Object model
  (Clause 12), services (Clause 15), BACnet/IP (Annex J), device profiles
  (Annex L). Purchase / preview via the [ASHRAE store](https://www.ashrae.org/technical-resources/standards-and-guidelines).
- **What is BACnet?** - Chipkin's introduction:
  <https://docs.chipkin.com/protocols/bacnet/>.
- **CAS BACnet Stack** - product page and documentation:
  <https://store.chipkin.com/services/stacks/bacnet-stack>.
- **CAS BACnet Explorer** - client for testing this device:
  <https://store.chipkin.com/products/tools/cas-bacnet-explorer>.
- **Shared helper used by this example** - [`src/common/README.md`](src/common/README.md).
- **Vendored adapter notes** - [`src/cas_bacnet_stack/README.md`](src/cas_bacnet_stack/README.md).

See also [TUTORIAL.md](TUTORIAL.md), [docs/PICS.md](docs/PICS.md),
[CHANGELOG.md](CHANGELOG.md), and [AGENTS.md](AGENTS.md).
