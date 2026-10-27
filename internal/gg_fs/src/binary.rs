use std::ffi::OsStr;
use std::io::Read;
use std::path::Path;

use crate::error::{Context, Result};
use crate::file::File;

const OP: &str = "read";

/// The number of leading bytes examined for content.
const SNIFF_LEN: usize = 8 * 1024;

/// Byte order marks of UTF-16 and UTF-32 text, which contains NUL bytes.
const TEXT_BOMS: [&[u8]; 3] = [&[0xff, 0xfe], &[0xfe, 0xff], &[0x00, 0x00, 0xfe, 0xff]];

/// Extensions of files that are binary without being opened.
const BINARY_EXTENSIONS: [&str; 208] = [
    "3g2", "3gp", "7z", "8svx", "a", "aa", "aac", "aax", "ace", "act", "adf", "afa", "aiff", "aml",
    "amr", "amv", "ape", "apk", "ar", "arc", "arj", "au", "avi", "awv", "b1", "b6z", "ba", "bh",
    "bin", "bmp", "bz2", "cab", "car", "cfs", "cpio", "cpt", "dar", "db", "dct", "dd", "dgc",
    "dll", "dmg", "drc", "dss", "dvf", "ear", "ecc", "efi", "exe", "f", "f4a", "f4b", "f4p", "f4v",
    "flac", "flv", "fpx", "gca", "gif", "gifv", "gsm", "gz", "ha", "hki", "ice", "icns", "ico",
    "ide", "iklax", "iso", "ivs", "j2c", "j2k", "jar", "jfif", "jif", "jp2", "jpeg", "jpg", "jpx",
    "kgb", "lbr", "lha", "lz", "lzh", "lzma", "lzo", "lzx", "m2ts", "m4a", "m4b", "m4p", "m4v",
    "mar", "mkv", "mmdb", "mmf", "mng", "mo", "mogg", "mov", "mp2", "mp3", "mp4", "mpc", "mpe",
    "mpeg", "mpg", "mpv", "msv", "mts", "mxf", "nsf", "nsv", "o", "oga", "ogg", "ogv", "opus",
    "pak", "paq6", "paq7", "paq8", "par", "par2", "partimg", "pcd", "pdf", "pea", "pim", "pit",
    "png", "pyc", "qda", "qt", "ra", "rar", "raw", "rev", "rk", "rm", "rmvb", "roq", "rz", "s7z",
    "sbx", "sda", "sea", "sen", "ser", "sfark", "sfx", "shar", "shk", "sit", "sitx", "sldasm",
    "slddrt", "slddrw", "sldprt", "sln", "sqlite", "sqx", "suo", "svg", "svi", "sz", "tar", "tbz2",
    "tgz", "tif", "tiff", "tlz", "tta", "twbw", "txz", "uc", "uc0", "uc1", "uc2", "uca", "ucn",
    "ue2", "uha", "ur2", "vob", "vox", "war", "wav", "webm", "wim", "wmv", "wv", "xar", "xcf",
    "xls", "xlsx", "xp3", "xz", "yuv", "yz1", "z", "zip", "zipx", "zoo", "zpaq", "zz",
];

/// Returns `true` if the file at `path` is binary.
///
/// A file with a known binary extension is binary without being opened.
///
/// # Errors
///
/// This function will return an error if the file cannot be opened or read.
pub fn is_binary(path: impl AsRef<Path>) -> Result<bool> {
    let path = path.as_ref();
    if has_binary_extension(path) {
        Ok(true)
    } else {
        let mut head = Vec::with_capacity(SNIFF_LEN);
        File::open(path)?.take(SNIFF_LEN as u64).read_to_end(&mut head).context(OP, path)?;
        Ok(is_binary_bytes(&head))
    }
}

/// Returns `true` if `head` is binary.
///
/// `head` is binary if it contains a NUL byte, unless it is UTF-16 or UTF-32
/// text.
pub fn is_binary_bytes(head: &[u8]) -> bool {
    let head = head.get(..SNIFF_LEN).unwrap_or(head);
    !TEXT_BOMS.iter().any(|bom| head.starts_with(bom)) && head.contains(&0)
}

fn has_binary_extension(path: &Path) -> bool {
    path.extension().and_then(OsStr::to_str).is_some_and(|extension| {
        BINARY_EXTENSIONS.iter().any(|b| b.eq_ignore_ascii_case(extension))
    })
}
