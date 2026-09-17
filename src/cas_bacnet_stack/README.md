# `cas_bacnet_stack/` - vendored Rust adapter

The CAS BACnet Stack's Rust FFI binding surface, copied (not referenced) from
`submodules/cas-bacnet-stack/adapters/rust/` - the same vendoring convention
the C#/Python editions of this series use for their own adapter directories.

## What's here

| File | What it is |
|---|---|
| `cas_bacnet_stack_adapters.rs` | GENERATED (by the stack's `ci_scripts/generate-rust-adapter.js`, from `source/CASBACnetStackDLL.h`, 231 exports). Every export is a `pub fn` that does a `libloading::Symbol` lookup + call inside `unsafe`, returning `BACnetResult<T> = Result<T, Box<dyn std::error::Error>>`. **One function is hand-edited from the generated original** - `load_library()` - see below. |
| `property_buffer_helper.rs` | Hand-written (not generated). Pack-helpers for the little-endian `propertyArrays` byte buffers consumed by `Send*` client-role calls (`SendReadProperty`, `SendWriteProperty`, `SendCreateObject`, ...). Vendored for parity with the adapter's expected file set, same precedent as the C#/Python editions' unused pack-helper files - see "Why this file is here but unused" below. |

Not vendored: `property_buffer_helper_selftest.rs`. It is a standalone test
harness for `property_buffer_helper.rs` with its own `main()`, not something
`main.rs` or any other file in this crate imports - there is nothing to point
it at once it is not wired into a test target here.

## The one deliberate edit: `load_library()`

`cas_bacnet_stack_adapters.rs`'s generated `load_library()` reads:

```rust
fn load_library() -> libloading::Library {
    unsafe {
        libloading::Library::new(
            "./bin/CASBACnetStack_x64_Debug".to_owned() + get_extension(env::consts::OS),
        )
        .unwrap()
    }
}
```

Two problems for this example: it names the **Debug**-configuration DLL (this
example only ever builds/ships **Release** - see "Why only a Release build" in
the top-level README.md, same reasoning as the C#/Python editions), and it
resolves `./bin/...` **relative to the process's current working directory**,
which breaks unless you happen to launch the executable from a directory that
has a `bin` subfolder next to the DLL.

Because this file is a **vendored copy**, not a reference to the submodule, we
are free to edit this one function. The vendored copy in this directory
instead:

1. Names `CASBACnetStack_x64_Release{.dll,.so,.dylib}` (platform-appropriate
   extension via the existing `get_extension()` helper).
2. Resolves that filename **next to the running executable**
   (`std::env::current_exe()`'s parent directory) - the same place
   README.md/CI copy the built native library to, and the same convention the
   C#/Python editions' native-library resolution uses.
3. Falls back to the bare filename (letting the OS's normal shared-library
   search path - `PATH`/`LD_LIBRARY_PATH`/rpath - have a try) if step 2 fails,
   rather than panicking immediately.
4. Still panics (with a descriptive message) if both fail - `load_library()`
   has no `Result`-returning signature to change (it backs a
   `once_cell::sync::Lazy<libloading::Library>`, which requires an infallible
   initializer). `main.rs` wraps its first call into the adapter in
   `std::panic::catch_unwind` specifically to turn that panic into the same
   kind of clean, actionable error message the C++/C#/Python editions print on
   a missing native library - see `main.rs`'s "Load the CAS BACnet Stack"
   section.

See the comment directly above `load_library()` in
`cas_bacnet_stack_adapters.rs` for the same explanation in place.

## Why `property_buffer_helper.rs` is here but unused

Same reasoning as the C#/Python editions' equivalent unused file: it packs
buffer-shaped `Send*` calls (ReadProperty/WriteProperty/CreateObject **as a
client**) that this read-only B-SS example never makes - a Smart Sensor only
ever *answers* ReadProperty, it never *initiates* one. It is vendored for
parity with the adapter's expected file set and compiles in unused
(`#[allow(dead_code)]` on its `mod` declaration in `mod.rs`) - a documented,
deliberate choice, not an oversight.

## Not CI-verified upstream

`cas_bacnet_stack_adapters.rs`'s own header says it: **no CI gate in the
`cas-bacnet-stack` repository compiles this file.** It was verified to
`cargo check` clean exactly once, by hand, on 2026-09-12 (issue #1483),
against the adapter's own standalone `Cargo.toml` (where it is built as a
`[lib]` crate). This example's own build (`cargo build --release` in the CI
workflow - see `.github/workflows/release.yml`) is, as far as this project is
aware, **the first automated compilation gate the Rust adapter has ever had**
- see the top-level `CHANGELOG.md` and `AGENTS.md` for the same note. Treat
any *future* change to the vendored files above as unverified again until
this example's CI (or another automated Rust build) re-checks it.
