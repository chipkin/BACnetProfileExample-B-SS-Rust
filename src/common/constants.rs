// SPDX-License-Identifier: CC0-1.0
// Public-domain example code (CC0) - see ../../LICENSE.

//! cas_bacnet_stack_example_constants.rs
//! =========================================================================
//! The BACnet enumeration values this example project needs.
//!
//! UNLIKE the C#/Python editions of this file, this one cannot lean on the
//! vendored adapter for most of these values: `cas_bacnet_stack_adapters.rs`
//! is a pure FFI binding layer (function signatures only) and defines **no**
//! `OBJECT_TYPE_*` / `PROPERTY_IDENTIFIER_*` / `SERVICES_SUPPORTED_*`
//! constants at all - the C# adapter's `CASBACnetStackAdapter.cs` and
//! Python's dict-based lookups both have an "Enumerations" section the Rust
//! generator does not (yet) emit. So every value below is defined here, not
//! reused from the adapter - this file is the complete set, not just the
//! handful the C#/Python editions had to add on top of their adapters.
//!
//! Every value matches the BACnet standard (ANSI/ASHRAE 135) and the CAS
//! BACnet Stack enumerations, and every constant name matches the
//! C++/C#/Python editions of this file
//! (`common/CASBACnetStackExampleConstants.h` in `BACnetProfileExample-B-SS-CPP`,
//! `common/CASBACnetStackExampleConstants.cs` in `-CS`,
//! `common/cas_bacnet_stack_example_constants.py` in `-Python`) so the
//! languages diff 1:1. Add more as your own project needs them.
//! =========================================================================

// -- BACnet object types (Object_Type enumeration) --------------------------
pub const OBJECT_TYPE_ANALOG_INPUT: u16 = 0;
pub const OBJECT_TYPE_BINARY_INPUT: u16 = 3;
pub const OBJECT_TYPE_DEVICE: u16 = 8;
pub const OBJECT_TYPE_MULTI_STATE_INPUT: u16 = 13;
pub const OBJECT_TYPE_NETWORK_PORT: u16 = 56;

// -- BACnet property identifiers (Property_Identifier enumeration) ----------
pub const PROPERTY_IDENTIFIER_OBJECT_NAME: u32 = 77;
#[allow(dead_code)]
pub const PROPERTY_IDENTIFIER_OBJECT_TYPE: u32 = 79;
pub const PROPERTY_IDENTIFIER_PRESENT_VALUE: u32 = 85;
pub const PROPERTY_IDENTIFIER_DESCRIPTION: u32 = 28;
pub const PROPERTY_IDENTIFIER_VENDOR_NAME: u32 = 121;
pub const PROPERTY_IDENTIFIER_VENDOR_IDENTIFIER: u32 = 120;
pub const PROPERTY_IDENTIFIER_MODEL_NAME: u32 = 70;
pub const PROPERTY_IDENTIFIER_FIRMWARE_REVISION: u32 = 44;
pub const PROPERTY_IDENTIFIER_APPLICATION_SOFTWARE_VERSION: u32 = 12;
pub const PROPERTY_IDENTIFIER_OUT_OF_SERVICE: u32 = 81;
pub const PROPERTY_IDENTIFIER_UNITS: u32 = 117;
pub const PROPERTY_IDENTIFIER_POLARITY: u32 = 84;
pub const PROPERTY_IDENTIFIER_NUMBER_OF_STATES: u32 = 74;
pub const PROPERTY_IDENTIFIER_STATE_TEXT: u32 = 110;
pub const PROPERTY_IDENTIFIER_APDU_LENGTH: u32 = 399;
pub const PROPERTY_IDENTIFIER_REFERENCE_PORT: u32 = 483;
pub const PROPERTY_IDENTIFIER_BACNET_IP_UDP_PORT: u32 = 412;
pub const PROPERTY_IDENTIFIER_BACNET_IP_MODE: u32 = 408;
pub const PROPERTY_IDENTIFIER_IP_ADDRESS: u32 = 400;
pub const PROPERTY_IDENTIFIER_IP_SUBNET_MASK: u32 = 411;
pub const PROPERTY_IDENTIFIER_IP_DEFAULT_GATEWAY: u32 = 401;

// -- BACnet engineering units (Engineering_Units enumeration) ---------------
// Full list: submodules/cas-bacnet-stack/source/BACnetEngineeringUnits.h
pub const ENGINEERING_UNITS_DEGREES_CELSIUS: u32 = 62;

// -- BACnet polarity (Polarity enumeration, for Binary objects) -------------
// Full list: submodules/cas-bacnet-stack/source/BACnetPolarity.h
pub const POLARITY_NORMAL: u32 = 0;

// -- BACnet/IP mode (BACnetIPMode enumeration, for the Network Port) --------
// Full list: submodules/cas-bacnet-stack/source/BACnetIPMode.h
pub const BACNET_IP_MODE_NORMAL: u32 = 0;

// -- BACnet services (Services_Supported enumeration) -----------------------
// Used with BACnetStack_SetServiceEnabled(). These are BIT NUMBERS, not
// service-choice values.
pub const SERVICE_READ_PROPERTY: u32 = 12;
pub const SERVICE_WHO_HAS: u32 = 33;
pub const SERVICE_WHO_IS: u32 = 34;
pub const SERVICE_I_HAVE: u32 = 27;
pub const SERVICE_I_AM: u32 = 26;

// -- Network Port object network type (BACnetNetworkType enumeration) -------
pub const NETWORK_PORT_NETWORK_TYPE_IPV4: u8 = 5;

// -- Network Port object protocol level (BACnetProtocolLevel enumeration) ---
pub const NETWORK_PORT_PROTOCOL_LEVEL_BACNET_APPLICATION: u8 = 2;

// The lowest protocol layer references this sentinel instead of another
// port. Also the default networkPortInstance for a single-port device that
// never calls AddNetworkPortObject (BACNET_NETWORK_PORT_DEFAULT).
pub const NETWORK_PORT_REFERENCE_PORT_NONE: u32 = 4194303;

// -- Network_Number_Quality (BACnetNetworkNumberQuality, cl. 12.56.11) ------
pub const NETWORK_NUMBER_QUALITY_UNKNOWN: u8 = 0;

// -- Character string encoding (cl. 20.2.9). 0 = UTF-8. ---------------------
pub const CHARACTER_STRING_ENCODING_UTF8: u8 = 0;

// -- BACnet error codes (Error_Code enumeration) -----------------------------
// Full list: submodules/cas-bacnet-stack/source/BACnetErrorCode.h
pub const ERROR_CODE_INVALID_ARRAY_INDEX: u32 = 42;

// -- This example's Network Port object instance -----------------------------
// Not a BACnet enumeration value - just this example's own configuration.
// Defined here (rather than as a `main.rs`-local `const`, as the C++/C#/
// Python editions do) because the transport callbacks in
// `cas_example_helper.rs` are plain `extern "C" fn` items that cannot
// capture a `main()`-local variable (see device_state.rs's module doc) and
// need this value at compile time to fill `out_network_port_instance`.
pub const NETWORK_PORT_INSTANCE: u32 = 1; // "Vermilion"
