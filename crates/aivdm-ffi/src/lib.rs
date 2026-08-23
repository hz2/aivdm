//! C FFI bindings for the `aivdm` AIS decoder.
//!
//! Exposes a decode-only subset of `aivdm`'s API: decode a single-fragment
//! `!AIVDM`/`!AIVDO` line into an opaque message handle, then read common
//! fields (MMSI, position, speed, course, heading, navigational status) off
//! it. See `include/aivdm.h` for the generated C declarations.
//!
//! This crate is a plain (non-`no_std`) library: it targets host
//! environments linking a shared or static C library, not the embedded
//! `no_alloc` use case `aivdm` itself supports. Embedded consumers should use
//! the `aivdm` Rust crate directly.

use std::ffi::{CStr, c_char};
use std::ptr;

use aivdm::AisMessage;

/// Opaque handle to a decoded AIS message, owned by the caller until freed
/// with [`aivdm_message_free`].
pub struct AivdmMessage(AisMessage);

/// Error codes returned by [`aivdm_decode_line`].
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AivdmError {
    /// No error.
    Ok = 0,
    /// The `line` pointer was NULL.
    NullInput = 1,
    /// `line` was not valid UTF-8.
    InvalidUtf8 = 2,
    /// The NMEA sentence itself was malformed (bad checksum, wrong field
    /// count, unsupported talker/formatter, invalid armor character).
    Nmea = 3,
    /// The sentence is one fragment of a multi-part message; this
    /// decode-only entry point only handles single-fragment sentences.
    IncompleteFragment = 4,
    /// A fragment-reassembly error (not produced by this entry point, which
    /// does no reassembly, but reserved for API symmetry).
    Fragment = 5,
    /// The payload was truncated or otherwise malformed at the bit level.
    Bit = 6,
    /// The message type is unknown, or a field held an invalid value.
    Message = 7,
}

impl From<aivdm::AisError> for AivdmError {
    fn from(e: aivdm::AisError) -> Self {
        match e {
            aivdm::AisError::Nmea(_) => Self::Nmea,
            aivdm::AisError::IncompleteFragment => Self::IncompleteFragment,
            aivdm::AisError::Fragment(_) => Self::Fragment,
            aivdm::AisError::Bit(_) => Self::Bit,
            // covers Message(_) and any future non_exhaustive variant
            _ => Self::Message,
        }
    }
}

/// Decodes a single, complete, single-fragment `!AIVDM`/`!AIVDO` line.
///
/// On success, returns a non-NULL handle (free it with
/// [`aivdm_message_free`] when done) and, if `out_error` is non-NULL, writes
/// [`AivdmError::Ok`] through it. On failure, returns NULL and, if
/// `out_error` is non-NULL, writes the reason through it.
///
/// # Safety
/// `line` must be NULL or a pointer to a NUL-terminated C string valid for
/// reads for the duration of this call. `out_error` must be NULL or a
/// pointer valid for a single `u8`-sized write.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aivdm_decode_line(
    line: *const c_char,
    out_error: *mut AivdmError,
) -> *mut AivdmMessage {
    let set_error = |code: AivdmError| {
        if !out_error.is_null() {
            // SAFETY: caller guarantees `out_error` is a valid write target or NULL.
            unsafe {
                out_error.write(code);
            }
        }
    };

    if line.is_null() {
        set_error(AivdmError::NullInput);
        return ptr::null_mut();
    }

    // SAFETY: caller guarantees `line` is a valid NUL-terminated C string.
    let Ok(line_str) = unsafe { CStr::from_ptr(line) }.to_str() else {
        set_error(AivdmError::InvalidUtf8);
        return ptr::null_mut();
    };

    match aivdm::decode_line(line_str) {
        Ok(msg) => {
            set_error(AivdmError::Ok);
            Box::into_raw(Box::new(AivdmMessage(msg)))
        }
        Err(e) => {
            set_error(AivdmError::from(e));
            ptr::null_mut()
        }
    }
}

/// Frees a message handle returned by [`aivdm_decode_line`]. Passing NULL is
/// a no-op.
///
/// # Safety
/// `msg` must be NULL or a pointer previously returned by
/// [`aivdm_decode_line`] that has not already been freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aivdm_message_free(msg: *mut AivdmMessage) {
    if !msg.is_null() {
        // SAFETY: caller guarantees `msg` is a live handle from `aivdm_decode_line`.
        drop(unsafe { Box::from_raw(msg) });
    }
}

/// The ITU-R M.1371 message type number (1-27) of a decoded message.
///
/// # Safety
/// `msg` must be NULL or a valid, non-freed handle from [`aivdm_decode_line`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aivdm_message_type(msg: *const AivdmMessage) -> u8 {
    // SAFETY: caller guarantees `msg` is a valid handle or NULL.
    match unsafe { msg.as_ref() } {
        Some(msg) => msg.0.message_type(),
        None => 0,
    }
}

/// How many times a repeater has relayed this message (0-3).
///
/// # Safety
/// `msg` must be NULL or a valid, non-freed handle from [`aivdm_decode_line`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aivdm_message_repeat_indicator(msg: *const AivdmMessage) -> u8 {
    // SAFETY: caller guarantees `msg` is a valid handle or NULL.
    match unsafe { msg.as_ref() } {
        Some(msg) => msg.0.repeat_indicator(),
        None => 0,
    }
}

/// The source station MMSI.
///
/// # Safety
/// `msg` must be NULL or a valid, non-freed handle from [`aivdm_decode_line`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aivdm_message_mmsi(msg: *const AivdmMessage) -> u32 {
    // SAFETY: caller guarantees `msg` is a valid handle or NULL.
    match unsafe { msg.as_ref() } {
        Some(msg) => msg.0.mmsi().raw(),
        None => 0,
    }
}

/// Reads the position, in decimal degrees, off a message.
///
/// Returns `true` and writes `*out_lat`/`*out_lon` if this message type
/// carries a position and it is marked available; returns `false` (leaving
/// the outputs untouched) otherwise.
///
/// # Safety
/// `msg` must be NULL or a valid, non-freed handle from
/// [`aivdm_decode_line`]. `out_lat` and `out_lon` must be valid pointers to
/// writable `f64`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aivdm_message_position(
    msg: *const AivdmMessage,
    out_lat: *mut f64,
    out_lon: *mut f64,
) -> bool {
    // SAFETY: caller guarantees `msg` is a valid handle or NULL.
    let Some(msg) = (unsafe { msg.as_ref() }) else {
        return false;
    };
    let Some(position) = msg.0.position() else {
        return false;
    };
    // SAFETY: caller guarantees `out_lat`/`out_lon` are valid write targets.
    unsafe {
        out_lat.write(position.latitude);
        out_lon.write(position.longitude);
    }
    true
}

/// Reads speed over ground, in knots, off a message.
///
/// Returns `true` and writes `*out_sog` if this message type carries a
/// speed and it is marked available; returns `false` otherwise.
///
/// # Safety
/// `msg` must be NULL or a valid, non-freed handle from
/// [`aivdm_decode_line`]. `out_sog` must be a valid pointer to a writable `f64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aivdm_message_sog_knots(
    msg: *const AivdmMessage,
    out_sog: *mut f64,
) -> bool {
    // SAFETY: caller guarantees `msg` is a valid handle or NULL.
    let Some(msg) = (unsafe { msg.as_ref() }) else {
        return false;
    };
    let Some(sog) = msg.0.sog_knots() else {
        return false;
    };
    // SAFETY: caller guarantees `out_sog` is a valid write target.
    unsafe {
        out_sog.write(sog);
    }
    true
}

/// Reads course over ground, in degrees, off a message.
///
/// Returns `true` and writes `*out_cog` if this message type carries a
/// course and it is marked available; returns `false` otherwise.
///
/// # Safety
/// `msg` must be NULL or a valid, non-freed handle from
/// [`aivdm_decode_line`]. `out_cog` must be a valid pointer to a writable `f64`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aivdm_message_cog_degrees(
    msg: *const AivdmMessage,
    out_cog: *mut f64,
) -> bool {
    // SAFETY: caller guarantees `msg` is a valid handle or NULL.
    let Some(msg) = (unsafe { msg.as_ref() }) else {
        return false;
    };
    let Some(cog) = msg.0.cog_degrees() else {
        return false;
    };
    // SAFETY: caller guarantees `out_cog` is a valid write target.
    unsafe {
        out_cog.write(cog);
    }
    true
}

/// Reads true heading, in whole degrees (0-359), off a message.
///
/// Returns `true` and writes `*out_heading` if this message type carries a
/// heading and it is marked available; returns `false` otherwise.
///
/// # Safety
/// `msg` must be NULL or a valid, non-freed handle from
/// [`aivdm_decode_line`]. `out_heading` must be a valid pointer to a writable `u16`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aivdm_message_heading_degrees(
    msg: *const AivdmMessage,
    out_heading: *mut u16,
) -> bool {
    // SAFETY: caller guarantees `msg` is a valid handle or NULL.
    let Some(msg) = (unsafe { msg.as_ref() }) else {
        return false;
    };
    let Some(heading) = msg.0.heading_degrees() else {
        return false;
    };
    // SAFETY: caller guarantees `out_heading` is a valid write target.
    unsafe {
        out_heading.write(heading);
    }
    true
}

/// Reads the raw navigational status code (0-15, ITU-R M.1371 Table 45) off
/// a message.
///
/// Returns `true` and writes `*out_status` if this message type carries a
/// navigational status; returns `false` otherwise.
///
/// # Safety
/// `msg` must be NULL or a valid, non-freed handle from
/// [`aivdm_decode_line`]. `out_status` must be a valid pointer to a writable `u8`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn aivdm_message_navigation_status(
    msg: *const AivdmMessage,
    out_status: *mut u8,
) -> bool {
    // SAFETY: caller guarantees `msg` is a valid handle or NULL.
    let Some(msg) = (unsafe { msg.as_ref() }) else {
        return false;
    };
    let Some(status) = msg.0.navigation_status() else {
        return false;
    };
    let status = status.to_raw();
    // SAFETY: caller guarantees `out_status` is a valid write target.
    unsafe {
        out_status.write(status);
    }
    true
}

/// The `aivdm` crate version, as a NUL-terminated, statically allocated
/// string (e.g. `"0.1.0"`). Do not free.
#[unsafe(no_mangle)]
pub extern "C" fn aivdm_version() -> *const c_char {
    const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");
    VERSION.as_ptr().cast::<c_char>()
}

// repeat_indicator/mmsi/position/sog_knots/cog_degrees/heading_degrees/
// navigation_status are all provided directly by `AisMessage` in the core
// crate (see `aivdm::message::AisMessage`) and called via
// `msg.0.<method>()` above, rather than duplicated here.
