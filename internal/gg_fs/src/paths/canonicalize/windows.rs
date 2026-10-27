use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;

const DIRECTORY_PATH_LIMIT: usize = 248;

/// Rewrites `\\?\C:\x` as `C:\x` and `\\?\UNC\host\share` as `\\host\share`.
/// Other verbatim paths are returned unchanged.
pub(super) fn simplify(path: PathBuf) -> PathBuf {
    let wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    let verbatim_unc: Vec<u16> = r"\\?\UNC\".encode_utf16().collect();
    let verbatim: Vec<u16> = r"\\?\".encode_utf16().collect();
    let simplified = if let Some(rest) = wide.strip_prefix(verbatim_unc.as_slice()) {
        let mut simplified: Vec<u16> = r"\\".encode_utf16().collect();
        simplified.extend_from_slice(rest);
        Some(simplified)
    } else {
        wide.strip_prefix(verbatim.as_slice()).filter(|rest| has_drive(rest)).map(<[u16]>::to_vec)
    };
    simplified
        .filter(|simplified| is_representable(simplified))
        .map_or(path, |simplified| PathBuf::from(OsString::from_wide(&simplified)))
}

fn has_drive(wide: &[u16]) -> bool {
    matches!(wide, [drive, colon, ..]
        if *colon == u16::from(b':')
            && u8::try_from(*drive).is_ok_and(|drive| drive.is_ascii_alphabetic()))
}

fn is_representable(wide: &[u16]) -> bool {
    let (dot, space, separator) = (u16::from(b'.'), u16::from(b' '), u16::from(b'\\'));
    wide.len() < DIRECTORY_PATH_LIMIT
        && wide
            .split(|unit| *unit == separator)
            .all(|name| name.last().is_none_or(|last| *last != dot && *last != space))
}
