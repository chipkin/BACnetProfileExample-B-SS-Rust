//! Pack-helpers for the little-endian propertyArrays byte buffers consumed by
//! CASBACnetStack_SendReadProperty / SendWriteProperty / SendCreateObject / SendWriteGroup /
//! SendSubscribeCOVPropertyMultiple. See ../README.md for the canonical byte-maps and
//! known-answer hex vectors this module's self-test (`property_buffer_helper_selftest.rs`) is
//! checked against.
//!
//! Pure Rust, no dependency on `cas_bacnet_stack_adapters.rs`'s `libloading`/`once_cell` — copy
//! this file alongside the adapter into your own project.
//!
//! # VALUES ARE PASSED AS TEXT, NOT RAW BINARY — EXCEPT [`WriteGroupChangeListEntry::value`] and
//! [`CreateObjectInitialValue::value`]
//!
//! (see their own docs below; #1594 converted SendWriteGroup's and SendCreateObject's value
//! encoding to raw binary on 2026-09-09, a hard cutover, ahead of the other buffers here.) For
//! everything else, a Real 72.5 is the 4 bytes `"72.5"`, not an IEEE-754 float. See the `value`
//! field docs on [`WriteElement`] below; getting this wrong is a silent wire-level bug that is
//! acknowledged normally by the far end. (The Node.js sibling,
//! adapters/node/@chipkin/cas-bacnet-stack/src/property-arrays.ts, carries the same warning —
//! keep the two in agreement.)

/// Highest BACnet primitive application datatype tag accepted by the write/create paths.
pub const WRITABLE_TAG_MAX: u8 = 12;

/// Highest BACnet write priority (cl. 19.2). 1..16 are valid; 0 means "no priority".
pub const WRITE_PRIORITY_MAX: u8 = 16;

const UINT32_MAX: u64 = 0xFFFF_FFFF;
const UINT16_MAX_LEN: usize = 0xFFFF;

/// Rejects any datatype that is not a BACnet primitive application tag (0..12). Constructed/
/// complex types are not supported by the v6 write/create path and are rejected by the stack at
/// decode time regardless — reject at pack time so the caller gets a clear error instead of a
/// wire-level one.
pub fn assert_writable_tag(tag: u8) -> Result<(), String> {
    if tag > WRITABLE_TAG_MAX {
        return Err(format!(
            "datatype tag {} is not a BACnet primitive application type (0..{}); \
             constructed/complex types are not supported by the write/create path",
            tag, WRITABLE_TAG_MAX
        ));
    }
    Ok(())
}

/// Rejects a numeric field that does not fit the fixed-width buffer writer backing it. Without
/// this, an out-of-range value silently truncates via the `as` casts used by the encoders below
/// instead of raising a usable error naming the offending field (issue #444 parity with the
/// Node.js port).
///
/// `path` is the element path for the message, e.g. `"elements[2].objectType"`. `value` is the
/// caller-supplied value. `max` is the highest accepted value (the writer's width, or a tighter
/// BACnet limit). `width_name` names the bound for the message, e.g. `"uint16"`.
pub fn assert_unsigned_field(path: &str, value: i64, max: u64, width_name: &str) -> Result<(), String> {
    if value < 0 || (value as u64) > max {
        return Err(format!("{} = {} is not an integer in 0..{} ({})", path, value, max, width_name));
    }
    Ok(())
}

/// Validates a field that uses a negative value as an "absent" sentinel (propertyArrayIndex,
/// priority). Negative passes through as absent; a non-negative value must fit its writer.
///
/// The sentinel itself is deliberately preserved — it is the documented convention on these
/// interfaces.
pub fn assert_sentinel_field(path: &str, value: i64, max: u64, width_name: &str) -> Result<(), String> {
    if value >= 0 && (value as u64) > max {
        return Err(format!("{} = {} is not an integer in 0..{} ({})", path, value, max, width_name));
    }
    Ok(())
}

/// Rejects a write priority outside BACnet's 1..16 (cl. 19.2), treating 0-or-negative as "absent"
/// per the documented convention on these interfaces.
pub fn assert_write_priority(path: &str, priority: i32) -> Result<(), String> {
    if priority > WRITE_PRIORITY_MAX as i32 {
        return Err(format!(
            "{} = {} exceeds the maximum BACnet write priority {} (cl. 19.2)",
            path, priority, WRITE_PRIORITY_MAX
        ));
    }
    Ok(())
}

/// Rejects a value payload whose length overflows the uint16 valueLength field that precedes it
/// on the wire. An over-long value would otherwise wrap silently.
pub fn assert_value_buffer(path: &str, value: &[u8]) -> Result<(), String> {
    if value.len() > UINT16_MAX_LEN {
        return Err(format!(
            "{}.length = {} exceeds the maximum {} (uint16 valueLength)",
            path,
            value.len(),
            UINT16_MAX_LEN
        ));
    }
    Ok(())
}

/// Rejects a non-finite covIncrement. `cov_increment` here is already `f32` (the wire width),
/// so — unlike the Node.js port, which narrows a wider `number`/`double` via `Math.fround` and
/// must check for saturation as a separate case — there is no narrowing step: any `f32` that is
/// not NaN/Infinity already fits the wire exactly.
fn assert_cov_increment(path: &str, cov_increment: f32) -> Result<(), String> {
    if !cov_increment.is_finite() {
        return Err(format!("{} = {} is not a finite number (IEEE-754 float)", path, cov_increment));
    }
    Ok(())
}

/// One ReadProperty / ReadPropertyMultiple property reference.
pub struct ReadElement {
    pub object_type: u16,
    pub object_instance: u32,
    pub property_identifier: u32,
    /// Negative means "no array index".
    pub property_array_index: i64,
}

/// Packs one or more ReadProperty / ReadPropertyMultiple elements — 15 bytes each:
/// `[0] flags u8 (bit0 = useArrayIndex) | [1..2] objectType u16 | [3..6] objectInstance u32 |
/// [7..10] propertyIdentifier u32 | [11..14] propertyArrayIndex u32`.
///
/// Returns the packed buffer; pass `elements.len()` as `propertyArraysCount`. Errors if any
/// element's `property_array_index` is a non-absent value that overflows u32 (`object_type`,
/// `object_instance` and `property_identifier` are already the exact wire width in Rust's type
/// system, so they cannot overflow and are not separately checked here).
pub fn encode_read_property_arrays(elements: &[ReadElement]) -> Result<Vec<u8>, String> {
    let mut buf = Vec::with_capacity(elements.len() * 15);
    for (i, e) in elements.iter().enumerate() {
        assert_sentinel_field(
            &format!("elements[{}].propertyArrayIndex", i),
            e.property_array_index,
            UINT32_MAX,
            "u32",
        )?;
        let use_index = e.property_array_index >= 0;
        buf.push(if use_index { 0x01u8 } else { 0x00u8 });
        buf.extend_from_slice(&e.object_type.to_le_bytes());
        buf.extend_from_slice(&e.object_instance.to_le_bytes());
        buf.extend_from_slice(&e.property_identifier.to_le_bytes());
        buf.extend_from_slice(&(if use_index { e.property_array_index as u32 } else { 0 }).to_le_bytes());
    }
    Ok(buf)
}

/// One WriteProperty / WritePropertyMultiple property write.
pub struct WriteElement {
    pub object_type: u16,
    pub object_instance: u32,
    pub property_identifier: u32,
    /// Negative means "no array index".
    pub property_array_index: i64,
    /// Zero or negative means "no priority".
    pub priority: i32,
    /// BACnet primitive application datatype tag (0..12).
    pub datatype: u8,
    /// The value as ASCII/UTF-8 TEXT — the characters a human would type — NOT a raw binary copy
    /// of the typed value.
    ///
    /// The stack parses these bytes with `BACnetAbstractSyntaxType::DecodeString`
    /// (source/BACnetAbstractSyntaxType.cpp), which does `ToValue<float>(std::string(...))` for
    /// Real and the equivalent for every other tag. So:
    ///
    /// ```text
    ///     Real 72.5     -> b"72.5"    (4 bytes of TEXT, not an IEEE-754 float)
    ///     Boolean true  -> b"1"
    ///     Unsigned 25   -> b"25"
    ///     Enumerated 3  -> b"3"
    ///     BitString     -> b"b10101"
    ///     OctetString   -> b"0x0102"
    ///     Null          -> b""
    /// ```
    ///
    /// Passing raw binary here is a SILENT wire-level bug for the numeric tags: the Boolean,
    /// Unsigned, Signed, Real and Double branches of DecodeString return success
    /// unconditionally, so unparseable bytes decode to 0 and the request is sent, encoded and
    /// acknowledged with no error at any layer. (OctetString and BitString do fail the decode and
    /// abort the send — which is why a test on those two tags will not reveal the mistake on the
    /// other five.)
    ///
    /// `datatype` selects which parse is applied; it does not describe the encoding of these
    /// bytes.
    pub value: Vec<u8>,
}

/// Packs one or more WriteProperty / WritePropertyMultiple elements — 19 bytes + value each:
/// `[0] flags u8 (bit0 useArrayIndex, bit1 usePriority) | [1..2] objectType u16 |
/// [3..6] objectInstance u32 | [7..10] propertyIdentifier u32 | [11..14] propertyArrayIndex u32 |
/// [15] dataType u8 | [16] priority u8 | [17..18] valueLength u16 | [19+] value`.
///
/// Returns the packed buffer; pass `elements.len()` as `propertyArraysCount` and
/// `buffer.len()` as `propertyArraysLength`. Errors if any element's datatype is not 0..12, its
/// `property_array_index` is a non-absent value overflowing u32, its `priority` exceeds
/// `WRITE_PRIORITY_MAX`, or its `value` exceeds the uint16 valueLength field.
pub fn encode_write_property_arrays(elements: &[WriteElement]) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    for (i, e) in elements.iter().enumerate() {
        assert_writable_tag(e.datatype)?;
        assert_sentinel_field(
            &format!("elements[{}].propertyArrayIndex", i),
            e.property_array_index,
            UINT32_MAX,
            "u32",
        )?;
        assert_write_priority(&format!("elements[{}].priority", i), e.priority)?;
        assert_value_buffer(&format!("elements[{}].value", i), &e.value)?;
        let use_index = e.property_array_index >= 0;
        let use_priority = e.priority > 0;
        let mut flags = 0u8;
        if use_index { flags |= 0x01; }
        if use_priority { flags |= 0x02; }
        buf.push(flags);
        buf.extend_from_slice(&e.object_type.to_le_bytes());
        buf.extend_from_slice(&e.object_instance.to_le_bytes());
        buf.extend_from_slice(&e.property_identifier.to_le_bytes());
        buf.extend_from_slice(&(if use_index { e.property_array_index as u32 } else { 0 }).to_le_bytes());
        buf.push(e.datatype);
        buf.push(if use_priority { e.priority as u8 } else { 0 });
        buf.extend_from_slice(&(e.value.len() as u16).to_le_bytes());
        buf.extend_from_slice(&e.value);
    }
    Ok(buf)
}

/// One CreateObject initial-property-value entry.
pub struct CreateObjectInitialValue {
    pub property_identifier: u32,
    /// Negative means "no array index".
    pub property_array_index: i64,
    /// Zero or negative means "no priority".
    pub priority: i32,
    /// BACnet primitive application datatype tag (0..12).
    pub datatype: u8,
    /// #1594 (2026-09-09): the value as RAW BINARY — NOT text, unlike [`WriteElement::value`].
    /// This is a hard cutover from the previous ASCII/UTF-8 text form; the stack now parses these
    /// bytes with `BACnetAbstractSyntaxType::DecodeBinary`. See `BACnetStack_SendCreateObject`'s
    /// "VALUE ENCODING" block in `CASBACnetStackDLL.h` for the full per-type byte layout — e.g.
    /// `72.5f32.to_le_bytes()` for a Real, not `b"72.5"`.
    pub value: Vec<u8>,
}

/// Packs one or more CreateObject initial-property-value elements — 13 bytes + value each (#961:
/// widened to add flags/property_array_index/priority so a caller can create an object with e.g.
/// a Weekly_Schedule[1] initial value or a commandable initial value at a priority - the field
/// order matches [`encode_write_property_arrays`]'s element above, minus object_type/
/// object_instance since an object being created has no identity yet):
/// `[0] flags u8 (bit0 useArrayIndex, bit1 usePriority) | [1..4] propertyIdentifier u32 |
/// [5..8] propertyArrayIndex u32 | [9] dataType u8 | [10] priority u8 |
/// [11..12] valueLength u16 | [13+] value`.
///
/// Returns the packed buffer; pass `values.len()` as `propertyArraysCount` and `buffer.len()` as
/// `propertyArraysLength`. Errors if any value's datatype is not 0..12, its `property_array_index`
/// is a non-absent value overflowing u32, its `priority` exceeds `WRITE_PRIORITY_MAX`, or its
/// `value` exceeds the uint16 valueLength field.
pub fn encode_create_object_initial_values(values: &[CreateObjectInitialValue]) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    for (i, v) in values.iter().enumerate() {
        assert_writable_tag(v.datatype)?;
        assert_sentinel_field(
            &format!("values[{}].propertyArrayIndex", i),
            v.property_array_index,
            UINT32_MAX,
            "u32",
        )?;
        assert_write_priority(&format!("values[{}].priority", i), v.priority)?;
        assert_value_buffer(&format!("values[{}].value", i), &v.value)?;
        let use_index = v.property_array_index >= 0;
        let use_priority = v.priority > 0;
        let mut flags = 0u8;
        if use_index { flags |= 0x01; }
        if use_priority { flags |= 0x02; }
        buf.push(flags);
        buf.extend_from_slice(&v.property_identifier.to_le_bytes());
        buf.extend_from_slice(&(if use_index { v.property_array_index as u32 } else { 0 }).to_le_bytes());
        buf.push(v.datatype);
        buf.push(if use_priority { v.priority as u8 } else { 0 });
        buf.extend_from_slice(&(v.value.len() as u16).to_le_bytes());
        buf.extend_from_slice(&v.value);
    }
    Ok(buf)
}

/// One SendWriteGroup change-list entry.
pub struct WriteGroupChangeListEntry {
    pub channel: u16,
    /// 0 = absent (valid overriding priorities are 1..16).
    pub overriding_priority: u8,
    /// BACnet primitive application datatype tag (0..12).
    pub value_datatype: u8,
    /// #1594 (2026-09-09): the value as RAW BINARY — NOT text, unlike [`WriteElement::value`].
    /// This is a hard cutover from the previous ASCII/UTF-8 text form; the stack now parses these
    /// bytes with `BACnetAbstractSyntaxType::DecodeBinary`. See `BACnetStack_SendWriteGroup`'s
    /// "VALUE ENCODING" block in `CASBACnetStackDLL.h` for the full per-type byte layout — e.g.
    /// `23.5f32.to_le_bytes()` for a Real, not `b"23.5"`.
    pub value: Vec<u8>,
}

/// Packs one or more SendWriteGroup change-list elements — 6 bytes + value each:
/// `[0..1] channel u16 | [2] overridingPriority u8 | [3] valueDatatype u8 |
/// [4..5] valueLength u16 | [6+] value`.
///
/// Returns the packed buffer; pass `entries.len()` as `changeListCount` and `buffer.len()` as
/// `changeListLength`. Errors if any entry's datatype is not 0..12, its `overriding_priority`
/// exceeds `WRITE_PRIORITY_MAX`, or its `value` exceeds the uint16 valueLength field.
///
/// Note: unlike [`WriteElement::priority`], `overriding_priority` is written straight through
/// with no absent-branch (and is already `u8`, so it can't even express a negative sentinel), so
/// it is validated with the plain unsigned-bound check (0..16) via [`assert_unsigned_field`], not
/// [`assert_write_priority`]'s negative-sentinel convention.
pub fn encode_write_group_change_list(entries: &[WriteGroupChangeListEntry]) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    for (i, e) in entries.iter().enumerate() {
        assert_writable_tag(e.value_datatype)?;
        assert_unsigned_field(
            &format!("entries[{}].overridingPriority", i),
            e.overriding_priority as i64,
            WRITE_PRIORITY_MAX as u64,
            "0 = absent, otherwise 1..16 per cl. 19.2",
        )?;
        assert_value_buffer(&format!("entries[{}].value", i), &e.value)?;
        buf.extend_from_slice(&e.channel.to_le_bytes());
        buf.push(e.overriding_priority);
        buf.push(e.value_datatype);
        buf.extend_from_slice(&(e.value.len() as u16).to_le_bytes());
        buf.extend_from_slice(&e.value);
    }
    Ok(buf)
}

/// One SendSubscribeCOVPropertyMultiple COV reference.
pub struct CovPropertyMultipleReference {
    pub monitored_object_type: u16,
    pub monitored_object_instance: u32,
    pub property_identifier: u32,
    pub use_property_array_index: bool,
    pub property_array_index: u32,
    pub use_cov_increment: bool,
    pub cov_increment: f32,
    pub timestamped: bool,
}

/// Packs one or more SendSubscribeCOVPropertyMultiple COV references — FIXED 19 bytes each (no
/// trailing value; a subscription reference carries no payload):
/// `[0] flags u8 (bit0 usePropertyArrayIndex, bit1 useCovIncrement, bit2 timestamped) |
/// [1..2] objectType u16 | [3..6] objectInstance u32 | [7..10] propertyIdentifier u32 |
/// [11..14] propertyArrayIndex u32 | [15..18] covIncrement f32 (IEEE-754, little-endian)`.
///
/// Returns the packed buffer; pass `references.len()` as `referenceCount` and `buffer.len()` as
/// `referencesLength`. Errors if a reference's `cov_increment` is non-finite while
/// `use_cov_increment` is set.
///
/// `property_array_index` here is already `u32` (not `i64` like [`ReadElement`]'s), so it cannot
/// carry a negative sentinel and needs no bounds check beyond what the type already guarantees;
/// it is simply zeroed when `use_property_array_index` is false, matching the sibling encoders'
/// "negative means no array index" convention without needing a wider type to express it.
pub fn encode_cov_property_multiple_references(references: &[CovPropertyMultipleReference]) -> Result<Vec<u8>, String> {
    let mut buf = Vec::with_capacity(references.len() * 19);
    for (i, r) in references.iter().enumerate() {
        if r.use_cov_increment {
            assert_cov_increment(&format!("references[{}].covIncrement", i), r.cov_increment)?;
        }
        let mut flags = 0u8;
        if r.use_property_array_index { flags |= 0x01; }
        if r.use_cov_increment { flags |= 0x02; }
        if r.timestamped { flags |= 0x04; }
        buf.push(flags);
        buf.extend_from_slice(&r.monitored_object_type.to_le_bytes());
        buf.extend_from_slice(&r.monitored_object_instance.to_le_bytes());
        buf.extend_from_slice(&r.property_identifier.to_le_bytes());
        buf.extend_from_slice(&(if r.use_property_array_index { r.property_array_index } else { 0 }).to_le_bytes());
        buf.extend_from_slice(&(if r.use_cov_increment { r.cov_increment } else { 0.0 }).to_le_bytes());
    }
    Ok(buf)
}
