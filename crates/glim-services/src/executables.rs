//! Bounded, read-only inspection of executable files; never run the inspected file.
use crate::binary::BytePreview;
use anyhow::{Context, Result, ensure};
use object::{Object, ObjectSection, ObjectSegment, read::macho::FatArch};
use std::{
    collections::BTreeSet,
    fmt::Write as _,
    io::{Read, Seek, SeekFrom},
    path::Path,
};
const LIMIT: u64 = 256 * 1024 * 1024;

pub fn supports(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let mut header = [0; 64];
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let Ok(count) = file.read(&mut header) else {
        return false;
    };
    if header.starts_with(b"MZ") {
        if count < 64 {
            return false;
        }
        let offset = u32::from_le_bytes(header[60..64].try_into().unwrap()) as u64;
        if !file
            .metadata()
            .is_ok_and(|m| m.is_file() && offset + 4 <= m.len())
        {
            return false;
        }
        let mut signature = [0; 4];
        return file.seek(SeekFrom::Start(offset)).is_ok()
            && file.read_exact(&mut signature).is_ok()
            && signature == *b"PE\0\0";
    }
    has_magic(&header[..count])
}
fn has_magic(bytes: &[u8]) -> bool {
    let Some(magic) = bytes.get(..4) else {
        return false;
    };
    if magic == b"\x7fELF" {
        return matches!(bytes.get(4), Some(1 | 2)) && matches!(bytes.get(5), Some(1 | 2));
    }
    match magic {
        b"\xfe\xed\xfa\xce" | b"\xce\xfa\xed\xfe" | b"\xfe\xed\xfa\xcf" | b"\xcf\xfa\xed\xfe" => {
            true
        }
        // Java class files share CAFEBABE. Require a plausible Mach-O fat
        // architecture count and first CPU type before routing to this reader.
        b"\xca\xfe\xba\xbe" | b"\xca\xfe\xba\xbf" => {
            let Some(count) = bytes.get(4..8) else {
                return false;
            };
            let Some(cpu) = bytes.get(8..12) else {
                return false;
            };
            let count = u32::from_be_bytes(count.try_into().unwrap());
            let cpu = u32::from_be_bytes(cpu.try_into().unwrap());
            (1..=32).contains(&count)
                && matches!(cpu, 7 | 12 | 18 | 0x01000007 | 0x0100000c | 0x01000012)
        }
        _ => false,
    }
}
pub fn read_preview(path: &Path) -> Result<BytePreview> {
    let mut file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file(),
        "Executable preview requires a regular file"
    );
    ensure!(
        metadata.len() <= LIMIT,
        "Executable preview is limited to 256 MiB"
    );
    let mut bytes = Vec::new();
    (&mut file).take(LIMIT + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= LIMIT,
        "Executable preview is limited to 256 MiB"
    );
    let mut text = format!(
        "Executable · Read-only inspection\nFile: {}\nSize: {} bytes\n\n",
        path.file_name().unwrap_or_default().to_string_lossy(),
        bytes.len()
    );
    match object::FileKind::parse(bytes.as_slice()).context("Invalid executable header")? {
        object::FileKind::MachOFat32 => {
            let fat = object::read::macho::MachOFatFile32::parse(bytes.as_slice())?;
            ensure!(
                (1..=32).contains(&fat.arches().len()),
                "Universal executable has too many architectures"
            );
            writeln!(
                text,
                "Container: Universal Mach-O · {} architectures\n",
                fat.arches().len()
            )?;
            for arch in fat.arches() {
                describe(arch.data(bytes.as_slice())?, &mut text)?;
            }
        }
        object::FileKind::MachOFat64 => {
            let fat = object::read::macho::MachOFatFile64::parse(bytes.as_slice())?;
            ensure!(
                (1..=32).contains(&fat.arches().len()),
                "Universal executable has too many architectures"
            );
            writeln!(
                text,
                "Container: Universal Mach-O · {} architectures\n",
                fat.arches().len()
            )?;
            for arch in fat.arches() {
                describe(arch.data(bytes.as_slice())?, &mut text)?;
            }
        }
        _ => describe(&bytes, &mut text)?,
    }
    writeln!(
        text,
        "\nHex preview · first {} bytes\nOffset    Hex bytes                                         ASCII",
        bytes.len().min(4096)
    )?;
    for (row, chunk) in bytes
        .iter()
        .take(4096)
        .copied()
        .collect::<Vec<_>>()
        .chunks(16)
        .enumerate()
    {
        write!(text, "{:08x}  ", row * 16)?;
        for index in 0..16 {
            match chunk.get(index) {
                Some(byte) => write!(text, "{byte:02x} ")?,
                None => text.push_str("   "),
            }
        }
        text.push(' ');
        for &byte in chunk {
            text.push(if byte.is_ascii_graphic() || byte == b' ' {
                char::from(byte)
            } else {
                '.'
            });
        }
        text.push('\n');
    }
    Ok(BytePreview {
        path: std::path::absolute(path)?,
        text,
    })
}
fn describe(bytes: &[u8], text: &mut String) -> Result<()> {
    let file = object::File::parse(bytes).context("Invalid or unsupported executable structure")?;
    ensure!(
        matches!(
            file.format(),
            object::BinaryFormat::MachO | object::BinaryFormat::Elf | object::BinaryFormat::Pe
        ),
        "Unsupported executable format"
    );
    writeln!(
        text,
        "Format: {:?}\nArchitecture: {:?}\nKind: {:?}\nWord size: {} bits\nByte order: {}\nEntry: 0x{:x}\n",
        file.format(),
        file.architecture(),
        file.kind(),
        if file.is_64() { 64 } else { 32 },
        if file.is_little_endian() {
            "little-endian"
        } else {
            "big-endian"
        },
        file.entry()
    )?;
    text.push_str("Segments (first 128)\nAddress           Size              File offset       File size         Name\n");
    for segment in file.segments().take(128) {
        let (offset, size) = segment.file_range();
        writeln!(
            text,
            "{:016x}  {:016x}  {:016x}  {:016x}  {}",
            segment.address(),
            segment.size(),
            offset,
            size,
            segment
                .name()
                .ok()
                .flatten()
                .unwrap_or("")
                .chars()
                .take(384)
                .collect::<String>()
        )?;
    }
    text.push_str("\nSections (first 256)\nAddress           Size              Name\n");
    for section in file.sections().take(256) {
        writeln!(
            text,
            "{:016x}  {:016x}  {}",
            section.address(),
            section.size(),
            section
                .name()
                .unwrap_or("<invalid name>")
                .chars()
                .take(384)
                .collect::<String>()
        )?;
    }
    text.push_str("\nImported libraries (up to 256 names; bounded header scan)\n");
    match libraries(&file) {
        Ok(libraries) => {
            if libraries.is_empty() {
                text.push_str("No library names available in these headers.\n");
            }
            for library in libraries {
                writeln!(text, "{library}")?;
            }
        }
        Err(_) => text.push_str("Dependency metadata unavailable or malformed.\n"),
    }
    text.push('\n');
    ensure!(
        text.len() <= 2 * 1024 * 1024,
        "Executable metadata exceeds preview budget"
    );
    Ok(())
}
fn libraries(file: &object::File<'_>) -> Result<BTreeSet<String>> {
    use object::read::elf::Dyn;
    let mut result = BTreeSet::new();
    let short = |bytes: &[u8]| {
        String::from_utf8_lossy(&bytes[..bytes.len().min(1536)])
            .chars()
            .take(384)
            .collect::<String>()
    };
    macro_rules! macho {
        ($file:expr) => {{
            let f = $file;
            let mut commands = f.macho_load_commands()?;
            for _ in 0..512 {
                let Some(command) = commands.next()? else {
                    break;
                };
                if command.cmd() != object::macho::LC_ID_DYLIB {
                    if let Some(dylib) = command.dylib()? {
                        result.insert(short(command.string(f.endian(), dylib.dylib.name)?));
                        if result.len() >= 256 {
                            break;
                        }
                    }
                }
            }
        }};
    }
    macro_rules! pe {
        ($file:expr) => {{
            if let Some(table) = $file.import_table()? {
                let mut descriptors = table.descriptors()?;
                for _ in 0..256 {
                    let Some(descriptor) = descriptors.next()? else {
                        break;
                    };
                    result.insert(short(
                        table.name(descriptor.name.get(object::LittleEndian))?,
                    ));
                }
            }
        }};
    }
    macro_rules! elf {
        ($file:expr) => {{
            let f = $file;
            let sections = f.elf_section_table();
            if let Some((dynamic, index)) = sections.dynamic(f.endian(), f.data())? {
                let strings = sections.strings(f.endian(), f.data(), index)?;
                for entry in dynamic.iter().take(4096) {
                    if entry.tag32(f.endian()) == Some(object::elf::DT_NEEDED) {
                        result.insert(short(entry.string(f.endian(), strings)?));
                        if result.len() >= 256 {
                            break;
                        }
                    }
                }
            }
        }};
    }
    match file {
        object::File::MachO32(f) => macho!(f),
        object::File::MachO64(f) => macho!(f),
        object::File::Pe32(f) => pe!(f),
        object::File::Pe64(f) => pe!(f),
        object::File::Elf32(f) => elf!(f),
        object::File::Elf64(f) => elf!(f),
        _ => {}
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn magic_distinguishes_text_and_java_from_native_binaries() {
        assert!(has_magic(b"\xcf\xfa\xed\xfe\x0c\0\0\x01\0\0\0\0"));
        assert!(has_magic(b"\x7fELF\x02\x01\x01\0"));
        assert!(!has_magic(b"MZ\0\0"));
        assert!(!has_magic(b"#!/bin/sh\n"));
        assert!(!has_magic(b"\xca\xfe\xba\xbe\0\0\0\x3d\0\x10\x07\0"));
        assert!(!has_magic(b"\xca\xfe\xba\xbe\0\0\0\x01\0\0\0\0"));
    }
    #[test]
    fn mz_text_is_not_misclassified_as_pe_and_pe_offsets_are_checked() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file");
        std::fs::write(&path, b"MZ is ordinary text, not an executable.").unwrap();
        assert!(!supports(&path));
        let mut bytes = vec![0; 128];
        bytes[..2].copy_from_slice(b"MZ");
        bytes[60..64].copy_from_slice(&64_u32.to_le_bytes());
        bytes[64..68].copy_from_slice(b"PE\0\0");
        std::fs::write(&path, &bytes).unwrap();
        assert!(supports(&path));
        bytes[60..64].copy_from_slice(&u32::MAX.to_le_bytes());
        std::fs::write(&path, &bytes).unwrap();
        assert!(!supports(&path));
    }
    #[test]
    fn minimal_arm64_macho_has_read_only_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("native");
        let bytes = [0xfeedfacf_u32, 0x0100000c, 0, 2, 0, 0, 0, 0]
            .into_iter()
            .flat_map(u32::to_le_bytes)
            .collect::<Vec<_>>();
        std::fs::write(&path, &bytes).unwrap();
        let preview = read_preview(&path).unwrap();
        assert!(preview.text.contains("MachO"));
        assert!(preview.text.contains("Aarch64"));
        assert!(preview.text.contains("64 bits"));
        assert!(preview.text.contains("Hex preview"));
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
    #[test]
    fn malformed_binary_is_rejected_without_running_or_modifying_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("native");
        let bytes = b"\xcf\xfa\xed\xfe\0\0\0\0\0\0\0\0";
        std::fs::write(&path, bytes).unwrap();
        assert!(supports(&path));
        assert!(read_preview(&path).is_err());
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        assert!(!supports(dir.path()));
    }
}
