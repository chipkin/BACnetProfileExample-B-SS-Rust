# Tutorial - extending and reviewing the B-SS example

[README.md](README.md) says what this example *is*. This document is the
*how*: how to extend it into your own device, who serves which property, how
to review the result for conformance, and what goes wrong when you get it
subtly right.

Read this once before you start changing `main.rs`. The most expensive
mistake in this example is silent, and the section it lives in is
[Add a second analog input](#add-a-second-analog-input).

- [Extending the example](#extending-the-example)
- [What each object type needs you to serve](#what-each-object-type-needs-you-to-serve)
- [Who serves what: the application or the stack?](#who-serves-what-the-application-or-the-stack)
- [Why callbacks read from `device_state::STATE`](#why-callbacks-read-from-device_statestate)
- [Interactive commands are a simplification](#interactive-commands-are-a-simplification)
- [Reviewing your device](#reviewing-your-device)
- [Troubleshooting](#troubleshooting)

## Extending the example

The example is intentionally small so it's easy to change.

**Change a sensor's value or name** - edit the constants / callbacks in
`main.rs` (e.g. `common::device_state::STATE.lock().unwrap().analog_input_1_value`,
or the `"Bronze"` string in `get_property_character_string`).

**Change the device identity before you ship** - vendor ID, vendor name,
model name, description, firmware revision and device name are all in the
`CHANGE ALL OF THIS BEFORE YOU SHIP` block at the top of `main.rs` §1, with a
per-field note on each saying what to change it to. That block is the
authoritative checklist; it is in the source rather than here so it cannot be
skipped by someone who only reads the code. Keep the application-owned socket
+ `cas_example_helper::register_common_callbacks()` shape, the habit of
checking every `BACnetStack_*` return value, and the
callbacks-read-from-a-shared-static pattern - those are part of the shape,
not the identity, and every sibling example (C++/C#/Python) keeps the first
two of those too.

### Add a second analog input

Read this whole recipe before starting - the last step is the one that is
easy to miss and the one BTL will fail you for.

> **Why there are four edits, not three - and why skipping one is SILENT.**
> Most of the `get_property_*` callbacks match on **both** object type *and*
> instance (`object_instance == ANALOG_INPUT_INSTANCE`), so a new instance
> falls through every one of them. `get_property_bool`'s `Out_Of_Service`
> branch is the exception: it matches on `property_identifier` (and
> `object_type`) alone, so `Out_Of_Service` works for a new instance for
> free.
>
> Here is the part that matters, and it is the opposite of what most people
> assume. Falling through a callback does **not** reliably produce an error.
> The stack errors only for the few properties it refuses to invent -
> `Present_Value`, `Number_Of_States`, `Relinquish_Default`, `Local_Date`,
> `Local_Time`, and a Network Port's `APDU_Length`. For everything else it
> **silently substitutes a default**:
>
> | Property | If you forget to serve it | Loud? |
> |---|---|:--:|
> | `Present_Value` | Error (`value-not-initialized`) | yes |
> | `Object_Name` | reads back as the literal string **`"undefined"`** | **no** |
> | `Units` | reads back as **`no-units` (95)** | **no** |
> | `Out_Of_Service` | served on property + type alone - works by accident | n/a |
>
> **Doesn't the `errorCode` out-parameter fix this?** Only if you use it, and
> only where it is right to. Each `get_property_*` callback receives a
> trailing `*mut u32 error_code` that the stack pre-seeds with `success` and
> reads only when you return `false`, so you *can* turn any decline into a
> chosen BACnet error. But writing an error code on the catch-all breaks the
> device: the stack's decline-and-fabricate path is what answers required
> properties an application is not expected to serve - the Device's
> `Max_APDU_Length_Accepted`, `APDU_Timeout` and `Number_Of_APDU_Retries`
> among them. Write `*error_code` only where *this device* knows the read is
> wrong; `main.rs` does it in exactly one place - `State_Text` with an
> out-of-range array index (`c::ERROR_CODE_INVALID_ARRAY_INDEX`).
>
> It is worse than "wrong value": the object's `Property_List` **still
> advertises `Units` (117)**. So the object actively claims to have the
> property, and then answers with a default. Nothing on the wire says you
> forgot anything.
>
> So a half-added object does not look broken; it looks **healthy**. Add two
> of them and both report `Object_Name "undefined"` - duplicate object names
> inside one device, which is a spec violation and a hard BTL failure that
> every scan tool will render as a perfectly good object. **"It scanned OK"
> is exactly the failure mode, not evidence against it.**

```rust
// 1) a new instance number (in §1 Configuration).
//    Naming: a second object of a type is "<Colour> 2" - so Analog Input 2 is
//    "Bronze 2", NOT a new colour. Each object TYPE owns one colour series-wide.
const ANALOG_INPUT_2_INSTANCE: u32 = 2; // "Bronze 2"
// ... and add an `analog_input_2_value: f32` field to `DeviceState` in
// device_state.rs, initialized in `DeviceState::new()` - it must live in the
// shared static, not a main()-local, because get_property_real cannot
// capture a local (see "Why callbacks read from device_state::STATE" below).

// 2) add the object (in main(), next to the other BACnetStack_AddObject calls).
//    Check the return, like every other stack call in this file.
if !bacnet::BACnetStack_AddObject(device_id, c::OBJECT_TYPE_ANALOG_INPUT, ANALOG_INPUT_2_INSTANCE).unwrap_or(false) {
    eprintln!("Error: Failed to add Analog Input 2 (Bronze 2).");
    return std::process::ExitCode::FAILURE;
}

// 3) serve its Present_Value + Object_Name:
//    get_property_real:              AI/2 + Present_Value -> *value = state.analog_input_2_value
//    get_property_character_string:  AI/2 + Object_Name    -> write_character_string("Bronze 2", ...)

// 4) DO NOT SKIP: serve its Units, in get_property_enumerated.
//    Units is a REQUIRED property of an Analog Input. The existing check reads
//    `object_instance == ANALOG_INPUT_INSTANCE`, which is instance 1 - so
//    without this, reading Analog Input 2's Units returns no-units(95) instead
//    of erroring, and the object is NON-CONFORMANT. It will still appear in
//    Object_List and its Present_Value will read back perfectly, so the device
//    looks healthy right up until BTL certification.
//    get_property_enumerated: AI/2 + Units -> *value = c::ENGINEERING_UNITS_DEGREES_CELSIUS
```

Then re-run the README's Verify steps **against Analog Input 2**, not just
Analog Input 1 - read every required property and **diff it against Analog
Input 1**. Any property that comes back `"undefined"`, `no-units`, or `0`
where object 1 returns something real is a step you missed. Because the
failure is silent (see the table above), this diff is the only thing that
catches it.

### Who serves what: applying it to a new object

There is no commandable-output section in this tutorial - **B-SS has no
outputs**: every object in this profile is a read-only input, so there is no
priority array, no `Relinquish_Default`, no `Set*` callback anywhere in
`main.rs`. If you need a commandable object (Analog Output, Binary Output,
Multi-State Output, or a Value object with `Priority_Array` enabled), that is
a *different* profile (B-SA or B-ASC) - see
[BACnetProfileExample-B-ASC-Node](https://github.com/chipkin/BACnetProfileExample-B-ASC-Node)
for the commandable-output model (`Commandable`, `CommandWrite`,
`CommandRelinquish`) and the `Set*` callback shape it requires (there is no
Rust edition of that example yet - port the shape using this example's
`extern "C" fn` conventions, keeping `Set*` callbacks free-standing and
`catch_unwind`-wrapped exactly like the `Get*` ones here). Bolting that model
onto this example without also enabling `WriteProperty`
(`SERVICES_SUPPORTED_WRITE_PROPERTY`) and the object's
`Priority_Array`/`Relinquish_Default` properties would not make the object
writable anyway - review
[§4 Application services supported](docs/PICS.md#4-application-services-supported)
in the PICS before you reach for it.

The recipe that *is* relevant to a Smart Sensor - adding another read-only
input of a **different** type - follows the same four-step shape as
[Add a second analog input](#add-a-second-analog-input) above: a new instance
number (in `DeviceState` if it needs to be mutable, or a plain `const` if it
does not), `BACnetStack_AddObject`, `Object_Name` + the type's other required
value property (`Polarity` for a Binary Input, `Number_Of_States` for a
Multi-State Input), and a diff against the existing object of that type.

## What each object type needs you to serve

The application must serve every REQUIRED property the stack does not
generate. It differs per type - this is the checklist, so you do not have to
infer it:

| Object type | You must serve | Plus |
|---|---|---|
| Analog Input | `Present_Value` (Real), `Object_Name`, `Units` | `Out_Of_Service` |
| Binary Input | `Present_Value` (Enumerated), `Object_Name` | `Polarity`, `Out_Of_Service` |
| Multi-State Input | `Present_Value` (Unsigned), `Object_Name` | `Number_Of_States`, `Out_Of_Service` |

`Out_Of_Service` is served once, in `get_property_bool`, matched on
`property_identifier` alone across all three input types plus the Network
Port - it is the one callback in this file that does NOT also match on
object type/instance, which is exactly why it is safe to leave unmatched: a
read-only sensor is never out of service, so the answer is always `false`.

## Who serves what: the application or the stack?

The single most common question when reading this file is "who answers this
property?" For Analog Input 1, the whole picture:

| Property | Served by | How |
|---|---|---|
| `Object_Identifier` | **stack** | generated from the object you added |
| `Object_Type` | **stack** | generated |
| `Object_List` | **stack** | generated (Device object) |
| `Property_List` | **stack** | generated |
| `Status_Flags` | **stack** | generated |
| `Event_State` | **stack**, sort of | no intrinsic alarming here, so nothing serves it - it reads `normal` only because `normal` is the enumeration's zero value and the stack substitutes a datatype default. Correct by coincidence, not design. |
| `Out_Of_Service` | **you** | `get_property_bool` - matched on property (+ type) only |
| `Present_Value` | **you** | `get_property_real` |
| `Object_Name` | **you** | `get_property_character_string` |
| `Units` | **you** | `get_property_enumerated` |

Every object, not just this one, is in [docs/PICS.md](docs/PICS.md).

Two more traps worth calling out explicitly, both silent:

- **Two `networkType`/Network Port instance identifiers, easy to confuse.**
  `c::NETWORK_PORT_NETWORK_TYPE_IPV4` (5, a *property* enumeration passed to
  `BACnetStack_AddNetworkPortObject`) is a *different* thing from the *link*
  identifier every transport call and `SendIAm` use - the Network Port
  object's own **instance** (`c::NETWORK_PORT_INSTANCE`, 1 in this example),
  which `cas_example_helper::register_common_callbacks()` and `send_i_am()`
  both read directly (as a compile-time constant, not a parameter - see
  "Why callbacks read from `device_state::STATE`" below for why that had to
  become a constant rather than an argument). Mixing them up compiles (both
  are plain integers) and can silently misroute a multi-port device; a
  single-port example like this one is forgiving of the mistake, which is
  exactly why it is worth naming.
- **The stack's own errorCode default is NOT "no error."**
  `get_property_character_string`'s `State_Text` branch is the one place
  this file writes `*error_code` - it is pre-seeded with `success`, so a
  `return false` there without setting it would put "Error Class PROPERTY,
  Error Code success" on the wire instead of `invalid-array-index`. Every
  other `false` returned in this file leaves `*error_code` untouched on
  purpose (see the callout in
  [Add a second analog input](#add-a-second-analog-input) above).

Going beyond this (WriteProperty, COV, alarms, scheduling) means implementing
a richer profile - see the series table in [README.md](README.md).

## Why callbacks read from `device_state::STATE`

If you have read the C++/C#/Python editions of this example, their
`Get*Property` callbacks read plain module-level/global variables (C++,
Python) or `static` fields (C#) directly - nothing unusual there. This
edition looks different for one specific, load-bearing reason:
`BACnetStack_RegisterCallbackGetProperty*` takes `Option<extern "C" fn(...)>`
- a **plain, non-capturing function pointer type**, not a closure type. A
Rust closure that captures anything (even by reference) is a distinct,
unnameable type that does not coerce to a bare `fn` pointer - the compiler
rejects it outright, it is not a runtime footgun you can work around with
`unsafe`.

So every `get_property_*` function in `main.rs` is a free-standing
`extern "C" fn` item, and the only state it can see is `static` state:
`common::device_state::STATE`, a `once_cell::sync::Lazy<Mutex<DeviceState>>`.
`main()` writes through the same lock (recording the discovered IP addressing
at start-up; nudging `analog_input_1_value` from the interactive-command
loop). If you add mutable per-object state (a second Analog Input's value, a
Binary Input you actually toggle), it goes in `DeviceState` in
`common/device_state.rs`, not a `main()`-local `let mut`.

This is also why `c::NETWORK_PORT_INSTANCE` is a compile-time constant in
`common/constants.rs` rather than a parameter threaded through
`register_common_callbacks(udp, network_port_instance)` the way the
C++/C#/Python editions do it: the transport callbacks in
`cas_example_helper.rs` are themselves plain `extern "C" fn` items and cannot
capture a parameter either.

## Interactive commands are a simplification

The C++ edition polls raw console keys (including arrow-key escape sequences)
every millisecond in its own loop. The C# edition uses .NET's built-in
non-blocking `Console.KeyAvailable`/`Console.ReadKey`. The Python edition
picks a platform-appropriate raw-mode poller (`msvcrt` on Windows,
`termios`/`tty`/`select` on POSIX).

Rust's standard library has **no** non-blocking single-keypress read on any
platform - genuinely raw terminal mode needs a platform-specific crate (e.g.
`crossterm`), which this example deliberately avoids to keep the dependency
list to exactly `libloading`/`once_cell`/`ctrlc` (see `Cargo.toml`). Instead,
`main.rs` reads **line-buffered** commands: type `up`, `down`, `h` or `q` and
press Enter. A background thread blocks on `stdin.read_line()` (there is no
portable non-blocking version of that in `std` either) and forwards completed
commands to the main loop over an `mpsc` channel, drained without blocking
(`try_recv()`) once per tick.

This also removes the need for a TTY check the other editions have: when
stdin is not an interactive console (a background process, the CI smoke
test, output piped to a file), `read_line()` simply returns `Ok(0)`
immediately (EOF), the reader thread exits quietly, and the main loop keeps
ticking the stack - Ctrl+C (via the `ctrlc` crate) still works either way.

## Reviewing your device

After you have changed anything, review it against the conformance statement
rather than against "it looked fine in the explorer":

1. Regenerate [docs/PICS.md](docs/PICS.md) after editing `docs/objects.json`
   (see [Keeping the PICS honest](#keeping-the-pics-honest) below). A ⚠ row is
   a required property nothing serves.
2. Read **every** property listed for **every** object with a BACnet client,
   and compare the value against the PICS. `"undefined"`, `no-units` and `0`
   are the three shapes a missed callback takes.
3. Diff a new object of a type against the existing one of that type. Anything
   that differs and shouldn't is a callback that matched on instance.
4. Send a **WriteProperty** to any object and confirm it is rejected - a B-SS
   Smart Sensor is read-only, and this example never registers a `Set*`
   callback, so the stack has nothing to consult and declines on its own.
5. Read `State_Text[4]` on Multi-State Input 1 (which only has 3 states) and
   confirm `invalid-array-index`, not an empty string.

### Keeping the PICS honest

`docs/PICS.md` is partly generated. `docs/objects.json` describes each object
and who serves which property; the series tool regenerates the object tables
from it plus the stack's own `docs/property-profile-reference.md` at the
pinned commit:

```bash
python tools/gen-objects-properties.py BACnetProfileExample-B-SS-Rust            # rewrite
python tools/gen-objects-properties.py BACnetProfileExample-B-SS-Rust --check    # fail if stale
```

(That tool lives in the example-series repository, not in this one. If you
only have this repository, edit the generated block by hand and keep it
matching the callbacks in `main.rs`.)

When you add an object or a property to `main.rs`, update `docs/objects.json`
in the same change and regenerate. The `app` list is what the callbacks
serve; `accepted` is for a required property you deliberately leave to the
stack's default, and each one needs a justification. Anything required, not
in `app` and not in `accepted`, comes out as a ⚠ row - that is a defect, not
a feature.

## Troubleshooting

| Symptom | Cause / fix |
|---------|-------------|
| Panic on start-up mentioning "failed to load ... CASBACnetStack_x64_Release" | The native library is missing from next to the built executable. See README.md [Build the native CAS BACnet Stack library](README.md#build-the-native-cas-bacnet-stack-library) - it is a separate step from `cargo build`. |
| Panic/error mentioning `CASBACnetStack_x64_Debug` | You are looking at stale instructions or a hand-edited copy of `load_library()` - this vendored copy always resolves the **Release** name; see [Why only a Release build](README.md#why-only-a-release-build) and `src/cas_bacnet_stack/README.md`. |
| The process exits immediately with an "unrecognized argument" error | Check your flags against `--help` - this example's CLI parser is hand-rolled (no `clap`) and only understands `--port`, `--deviceID`, `--help`, `--version`. |
| App prints *"could not bind UDP port 47808"* | Another BACnet program is already using 47808. Stop it, or run with `--port <n>`. |
| Client sends Who-Is but sees no I-Am | Firewall is blocking UDP 47808, or the client and device are on different subnets (Who-Is is a broadcast). Allow the port; test on the same subnet first. |
| A WriteProperty you sent is rejected | Correct behaviour - this device is read-only. There is no `BACnetStack_RegisterCallbackSetProperty*` call anywhere in `main.rs`; the stack declines every write without consulting the application. |
| Typing `up`/`down`/`h`/`q` does nothing | Remember to press **Enter** - see [Interactive commands are a simplification](#interactive-commands-are-a-simplification); this edition is line-buffered, not raw single-keypress like the other editions. |
