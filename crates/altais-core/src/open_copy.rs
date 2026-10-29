//! Temporary decrypted copies for opening an attachment in its default
//! program.
//!
//! - Copies live in one folder in the user's temporary folder, each in its
//!   own random subfolder (two attachments with the same name do not clash).
//! - The folder is emptied when the archive is locked, when ALTAIS closes
//!   and when it starts (in case it did not close properly).
//! - Files that would run as programs (`.exe`, `.bat`, `.js`, …) are never
//!   opened; they can only be saved.
//! - On Windows each copy is marked as coming from the internet (Mark of the
//!   Web), so Office opens it in Protected View and Windows warns before
//!   running anything embedded.

use std::fs;
use std::path::{Path, PathBuf};

/// Extensions Windows (or an installed runtime) executes when opened.
const RUNNABLE: &[&str] = &[
    "ade",
    "adp",
    "app",
    "appx",
    "appxbundle",
    "application",
    "appref-ms",
    "asp",
    "bas",
    "bat",
    "bgi",
    "cab",
    "cer",
    "chm",
    "cmd",
    "cnt",
    "com",
    "cpl",
    "crt",
    "csh",
    "der",
    "diagcab",
    "exe",
    "fxp",
    "gadget",
    "grp",
    "hlp",
    "hpj",
    "hta",
    "htc",
    "inf",
    "ins",
    "isp",
    "its",
    "jar",
    "jnlp",
    "js",
    "jse",
    "ksh",
    "lnk",
    "mad",
    "maf",
    "mag",
    "mam",
    "maq",
    "mar",
    "mas",
    "mat",
    "mau",
    "mav",
    "maw",
    "mcf",
    "mda",
    "mdb",
    "mde",
    "mdt",
    "mdw",
    "mdz",
    "msc",
    "msh",
    "msh1",
    "msh2",
    "mshxml",
    "msh1xml",
    "msh2xml",
    "msi",
    "msix",
    "msixbundle",
    "msp",
    "mst",
    "msu",
    "ops",
    "osd",
    "pcd",
    "pif",
    "pl",
    "plg",
    "prf",
    "prg",
    "printerexport",
    "ps1",
    "ps1xml",
    "ps2",
    "ps2xml",
    "psc1",
    "psc2",
    "psd1",
    "psdm1",
    "pssc",
    "pst",
    "py",
    "pyc",
    "pyo",
    "pyw",
    "pyz",
    "pyzw",
    "rb",
    "reg",
    "scf",
    "scr",
    "sct",
    "settingcontent-ms",
    "shb",
    "shs",
    "sh",
    "theme",
    "udl",
    "url",
    "vb",
    "vbe",
    "vbp",
    "vbs",
    "vhd",
    "vhdx",
    "vsmacros",
    "vsw",
    "webpnp",
    "website",
    "ws",
    "wsb",
    "wsc",
    "wsf",
    "wsh",
    "xbap",
    "xll",
    "xnk",
    "library-ms",
    "search-ms",
    "searchconnector-ms",
    "iso",
    "img",
];

/// Whether a file with this name may be opened in its default program.
pub fn safe_to_open(name: &str) -> bool {
    let name = name.trim_end_matches(['.', ' ']).to_ascii_lowercase();
    match name.rsplit_once('.') {
        Some((_, ext)) => !RUNNABLE.contains(&ext),
        // No extension: Windows would ask which program to use; refuse.
        None => false,
    }
}

/// A file name safe for the file system: the last path component, without
/// characters Windows forbids, without trailing dots or spaces, not a
/// reserved device name, at most 120 characters.
pub fn file_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let mut clean: String = base
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*') {
                '_'
            } else {
                c
            }
        })
        .collect();
    while clean.ends_with(['.', ' ']) {
        clean.pop();
    }
    let stem = clean.split('.').next().unwrap_or("").to_ascii_uppercase();
    const RESERVED: &[&str] = &[
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
        "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if clean.is_empty() || RESERVED.contains(&stem.trim()) {
        clean.insert_str(0, "attachment_");
    }
    if clean.chars().count() > 120 {
        // Keep the extension.
        let ext = clean
            .rsplit_once('.')
            .map(|(_, e)| e.to_owned())
            .filter(|e| e.chars().count() <= 16);
        let keep = 120 - ext.as_ref().map_or(0, |e| e.chars().count() + 1);
        let mut short: String = clean.chars().take(keep).collect();
        if let Some(ext) = ext {
            short.push('.');
            short.push_str(&ext);
        }
        clean = short;
    }
    clean
}

/// Writes `data` as `name` into a new subfolder of `root` and returns the
/// file's path.
pub fn write(root: &Path, name: &str, data: &[u8]) -> std::io::Result<PathBuf> {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).map_err(std::io::Error::other)?;
    let folder = root.join(bytes.iter().map(|b| format!("{b:02x}")).collect::<String>());
    fs::create_dir_all(&folder)?;
    let path = folder.join(file_name(name));
    fs::write(&path, data)?;
    #[cfg(windows)]
    {
        // Mark of the Web; failure (e.g. a FAT USB stick as TEMP) is not fatal.
        let mut stream = path.clone().into_os_string();
        stream.push(":Zone.Identifier");
        let _ = fs::write(stream, "[ZoneTransfer]\r\nZoneId=3\r\n");
    }
    Ok(path)
}

/// Deletes all copies; files another program still holds open stay and
/// are deleted next time.
pub fn clear(root: &Path) {
    let _ = fs::remove_dir_all(root);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn programs_are_never_opened() {
        for name in [
            "setup.exe",
            "RUN.BAT",
            "x.js",
            "a.pdf.exe",
            "link.lnk",
            "e.exe.",
            "e.exe ",
            "noext",
        ] {
            assert!(!safe_to_open(name), "{name}");
        }
        for name in [
            "zmluva.pdf",
            "foto.JPG",
            "tabuľka.xlsx",
            "trasa.gpx",
            "poznámky.txt",
        ] {
            assert!(safe_to_open(name), "{name}");
        }
    }

    #[test]
    fn names_are_made_safe() {
        assert_eq!(file_name("..\\..\\Windows\\evil.pdf"), "evil.pdf");
        assert_eq!(file_name("a/b/c.txt"), "c.txt");
        assert_eq!(file_name("what?<>.pdf"), "what___.pdf");
        assert_eq!(file_name("CON.txt"), "attachment_CON.txt");
        assert_eq!(file_name("report. . "), "report");
        assert_eq!(file_name(""), "attachment_");
        let long = format!("{}.pdf", "ž".repeat(300));
        let short = file_name(&long);
        assert_eq!(short.chars().count(), 120);
        assert!(short.ends_with(".pdf"));
    }

    #[test]
    fn copies_are_written_apart_and_cleared() {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).unwrap();
        let root = std::env::temp_dir().join(format!(
            "altais-open-test-{}",
            bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ));
        let a = write(&root, "a.txt", b"one").unwrap();
        let b = write(&root, "a.txt", b"two").unwrap();
        assert_ne!(a, b);
        assert_eq!(fs::read(&a).unwrap(), b"one");
        assert!(a.starts_with(&root));
        clear(&root);
        assert!(!root.exists());
    }
}
