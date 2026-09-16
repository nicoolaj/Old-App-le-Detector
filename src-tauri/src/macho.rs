// SPDX-License-Identifier: MIT

//! Lecture des architectures d'un binaire Mach-O directement depuis son en-tête,
//! sans dépendre de `file` ou `lipo` (qui exigent les Xcode CLT).
//!
//! Références : `<mach-o/loader.h>` et `<mach-o/fat.h>` du SDK macOS.

use std::io::Read;

// Magic numbers (mach-o/loader.h, mach-o/fat.h)
const MH_MAGIC: u32 = 0xfeedface; // Mach-O 32 bits, little-endian hôte
const MH_CIGAM: u32 = 0xcefaedfe; // Mach-O 32 bits, big-endian hôte
const MH_MAGIC_64: u32 = 0xfeedfacf; // Mach-O 64 bits, little-endian hôte
const MH_CIGAM_64: u32 = 0xcffaedfe; // Mach-O 64 bits, big-endian hôte
const FAT_MAGIC: u32 = 0xcafebabe; // Universel, big-endian sur le fichier
const FAT_CIGAM: u32 = 0xbebafeca;
const FAT_MAGIC_64: u32 = 0xcafebabf;
const FAT_CIGAM_64: u32 = 0xbfbafeca;

// cputype (mach/machine.h)
const CPU_TYPE_X86_64: u32 = 0x0100_0007;
const CPU_TYPE_ARM64: u32 = 0x0100_000c;
const CPU_TYPE_I386: u32 = 7;
const CPU_TYPE_ARM: u32 = 12;
const CPU_TYPE_POWERPC: u32 = 18;
const CPU_TYPE_POWERPC64: u32 = 0x0100_0012;

/// Nombre max d'octets nécessaires : en-tête fat 64 pour ~20 slices.
/// Largement suffisant, un binaire universel Apple en a rarement plus de 3-4.
const READ_LEN: usize = 8 + 20 * 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Arch {
    Arm64,
    X86_64,
    I386,
    Arm,
    PowerPc,
}

impl Arch {
    pub fn label(self) -> &'static str {
        match self {
            Arch::Arm64 => "arm64",
            Arch::X86_64 => "x86_64",
            Arch::I386 => "i386",
            Arch::Arm => "arm",
            Arch::PowerPc => "ppc",
        }
    }

    fn from_cputype(cputype: u32) -> Option<Arch> {
        match cputype {
            CPU_TYPE_ARM64 => Some(Arch::Arm64),
            CPU_TYPE_X86_64 => Some(Arch::X86_64),
            CPU_TYPE_I386 => Some(Arch::I386),
            CPU_TYPE_ARM => Some(Arch::Arm),
            CPU_TYPE_POWERPC | CPU_TYPE_POWERPC64 => Some(Arch::PowerPc),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Contient une slice arm64 (arm64e inclus) : tourne nativement sur Apple Silicon.
    Native,
    /// x86_64 sans arm64 : nécessite Rosetta 2.
    IntelOnly,
    /// Seulement i386/ppc/ppc64 : déjà inexécutable sur tout Mac récent.
    Obsolete,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Native => "native",
            Status::IntelOnly => "intel_only",
            Status::Obsolete => "obsolete",
        }
    }
}

pub fn classify(archs: &[Arch]) -> Status {
    if archs.contains(&Arch::Arm64) {
        Status::Native
    } else if archs.contains(&Arch::X86_64) {
        Status::IntelOnly
    } else {
        Status::Obsolete
    }
}

/// Lit les architectures présentes dans un binaire Mach-O (thin ou universel).
///
/// - `Err` : le fichier n'a pas pu être ouvert/lu (permissions, etc.) — appelant
///   doit le compter comme illisible, pas comme un script.
/// - `Ok(None)` : lu avec succès mais ce n'est pas un Mach-O reconnu (script,
///   fichier trop court, `.class` Java qui partage le magic fat...).
/// - `Ok(Some(archs))` : architectures trouvées.
pub fn read_archs(path: &std::path::Path) -> std::io::Result<Option<Vec<Arch>>> {
    let mut f = std::fs::File::open(path)?;
    let mut buf = [0u8; READ_LEN];
    let n = f.read(&mut buf)?;
    Ok(archs_from_header(&buf[..n]))
}

fn u32_at(buf: &[u8], off: usize, big_endian: bool) -> Option<u32> {
    let b: [u8; 4] = buf.get(off..off + 4)?.try_into().ok()?;
    Some(if big_endian {
        u32::from_be_bytes(b)
    } else {
        u32::from_le_bytes(b)
    })
}

fn archs_from_header(buf: &[u8]) -> Option<Vec<Arch>> {
    let magic = u32_at(buf, 0, false)?; // lu tel quel ; on compare aux variantes CIGAM/MAGIC
    match magic {
        MH_MAGIC | MH_CIGAM => {
            let big_endian = magic == MH_CIGAM;
            let cputype = u32_at(buf, 4, big_endian)?;
            Arch::from_cputype(cputype).map(|a| vec![a])
        }
        MH_MAGIC_64 | MH_CIGAM_64 => {
            let big_endian = magic == MH_CIGAM_64;
            let cputype = u32_at(buf, 4, big_endian)?;
            Arch::from_cputype(cputype).map(|a| vec![a])
        }
        FAT_MAGIC | FAT_CIGAM | FAT_MAGIC_64 | FAT_CIGAM_64 => fat_archs(buf, magic),
        _ => None,
    }
}

/// Les fichiers fat sont toujours stockés big-endian, y compris quand le magic
/// lu tel quel ressemble au CIGAM (lu depuis une machine little-endian, ce qui
/// est le cas de tous les Mac actuels).
fn fat_archs(buf: &[u8], magic: u32) -> Option<Vec<Arch>> {
    let is_64 = magic == FAT_MAGIC_64 || magic == FAT_CIGAM_64;
    let nfat_arch = u32_at(buf, 4, true)?;

    // Piège : CA FE BA BE est aussi le magic des .class Java, où les octets
    // suivants sont la version mineure/majeure du bytecode (majeure >= 45).
    // Un fat Mach-O Apple n'a jamais un nombre de slices à 3 chiffres.
    if nfat_arch == 0 || nfat_arch >= 30 {
        return None;
    }

    let entry_size = if is_64 { 32 } else { 20 };
    let mut out = Vec::with_capacity(nfat_arch as usize);
    for i in 0..nfat_arch as usize {
        let off = 8 + i * entry_size;
        let cputype = u32_at(buf, off, true)?;
        if let Some(a) = Arch::from_cputype(cputype) {
            out.push(a);
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thin_header(magic: u32, cputype: u32, big_endian: bool) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&magic.to_le_bytes()); // magic est toujours écrit "tel quel"
        let ct = if big_endian {
            cputype.to_be_bytes()
        } else {
            cputype.to_le_bytes()
        };
        v.extend_from_slice(&ct);
        v.extend_from_slice(&[0u8; 8]); // padding, non lu
        v
    }

    #[test]
    fn thin_x86_64_le() {
        let buf = thin_header(MH_MAGIC_64, CPU_TYPE_X86_64, false);
        assert_eq!(archs_from_header(&buf), Some(vec![Arch::X86_64]));
    }

    #[test]
    fn thin_arm64_le() {
        let buf = thin_header(MH_MAGIC_64, CPU_TYPE_ARM64, false);
        assert_eq!(archs_from_header(&buf), Some(vec![Arch::Arm64]));
    }

    #[test]
    fn thin_ppc_big_endian() {
        let buf = thin_header(MH_CIGAM, CPU_TYPE_POWERPC, true);
        assert_eq!(archs_from_header(&buf), Some(vec![Arch::PowerPc]));
        assert_eq!(classify(&[Arch::PowerPc]), Status::Obsolete);
    }

    fn fat_header(archs: &[u32]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(&FAT_MAGIC.to_le_bytes());
        v.extend_from_slice(&(archs.len() as u32).to_be_bytes());
        for cputype in archs {
            v.extend_from_slice(&cputype.to_be_bytes()); // cputype
            v.extend_from_slice(&[0u8; 16]); // cpusubtype, offset, size, align
        }
        v
    }

    #[test]
    fn fat_x86_64_plus_arm64() {
        let buf = fat_header(&[CPU_TYPE_X86_64, CPU_TYPE_ARM64]);
        let archs = archs_from_header(&buf).unwrap();
        assert_eq!(archs.len(), 2);
        assert!(archs.contains(&Arch::X86_64));
        assert!(archs.contains(&Arch::Arm64));
        assert_eq!(classify(&archs), Status::Native);
    }

    #[test]
    fn fat_x86_64_only_is_intel_only() {
        let buf = fat_header(&[CPU_TYPE_X86_64]);
        let archs = archs_from_header(&buf).unwrap();
        assert_eq!(classify(&archs), Status::IntelOnly);
    }

    #[test]
    fn java_class_file_is_not_a_fat_macho() {
        // CA FE BA BE suivi de version mineure/majeure de bytecode (ex: Java 17 = 61)
        let mut v = vec![0xCA, 0xFE, 0xBA, 0xBE];
        v.extend_from_slice(&0u16.to_be_bytes()); // minor
        v.extend_from_slice(&61u16.to_be_bytes()); // major (Java 17)
        v.extend_from_slice(&[0u8; 20]);
        assert_eq!(archs_from_header(&v), None);
    }

    #[test]
    fn shebang_script_is_none() {
        let buf = b"#!/bin/bash\necho hi\n".to_vec();
        assert_eq!(archs_from_header(&buf), None);
    }

    #[test]
    fn empty_file_is_none() {
        assert_eq!(archs_from_header(&[]), None);
    }

    #[test]
    fn too_short_for_cputype_is_none() {
        // Magic seul, pas assez d'octets pour lire le cputype.
        let buf = MH_MAGIC_64.to_le_bytes().to_vec();
        assert_eq!(archs_from_header(&buf), None);
    }

    /// `/usr/bin/file` est universel (x86_64 + arm64 + arm64e) sur tout macOS
    /// récent : bon canari qu'on lit vraiment le fichier sur disque.
    #[test]
    fn real_binary_on_disk() {
        let archs = read_archs(std::path::Path::new("/usr/bin/file"))
            .expect("lecture de /usr/bin/file")
            .expect("/usr/bin/file devrait être un Mach-O universel");
        assert!(archs.contains(&Arch::Arm64), "attendu arm64 dans {archs:?}");
    }
}
