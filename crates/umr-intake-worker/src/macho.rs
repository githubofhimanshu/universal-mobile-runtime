use byteorder::{BigEndian, LittleEndian, ReadBytesExt};
use std::io::Cursor;

use umr_core::{CpuArchitecture, DiagnosticError, EncryptionStatus, ErrorCode, UmrResult};

const MH_MAGIC: u32 = 0xfeedface;
const MH_CIGAM: u32 = 0xcefaedfe;
const MH_MAGIC_64: u32 = 0xfeedfacf;
const MH_CIGAM_64: u32 = 0xcffaedfe;
const FAT_MAGIC: u32 = 0xcafebabe;
const FAT_CIGAM: u32 = 0xbebafeca;

const CPU_TYPE_ARM: u32 = 12;
const CPU_TYPE_ARM64: u32 = 0x0100000c;
const CPU_TYPE_X86_64: u32 = 0x01000007;

const LC_ENCRYPTION_INFO: u32 = 0x21;
const LC_ENCRYPTION_INFO_64: u32 = 0x2c;
const LC_BUILD_VERSION: u32 = 0x32;
const PLATFORM_IOS_SIMULATOR: u32 = 7;

#[derive(Debug, Clone)]
pub struct MachOInspection {
    pub architectures: Vec<CpuArchitecture>,
    pub encryption: EncryptionStatus,
    pub is_simulator_build: bool,
}

/// Parses Mach-O binary headers to extract architecture, encryption (FairPlay cryptid), and build platform.
pub fn inspect_macho(bytes: &[u8]) -> UmrResult<MachOInspection> {
    if bytes.len() < 4 {
        return Err(DiagnosticError::new(
            ErrorCode::ErrIntakeParseFailure,
            "Executable file too small to be a valid Mach-O binary",
        ));
    }

    let magic = (&bytes[0..4]).read_u32::<BigEndian>().unwrap_or(0);

    match magic {
        FAT_MAGIC | FAT_CIGAM => parse_fat_binary(bytes, magic == FAT_MAGIC),
        _ => {
            // Check thin Mach-O (32-bit or 64-bit)
            let mut cursor = Cursor::new(bytes);
            let magic_le = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
            if magic_le == MH_MAGIC || magic_le == MH_CIGAM || magic_le == MH_MAGIC_64 || magic_le == MH_CIGAM_64 {
                let is_64 = magic_le == MH_MAGIC_64 || magic_le == MH_CIGAM_64;
                let is_little_endian = magic_le == MH_MAGIC || magic_le == MH_MAGIC_64;
                parse_thin_macho(bytes, 0, is_64, is_little_endian)
            } else {
                Err(DiagnosticError::new(
                    ErrorCode::ErrIntakeParseFailure,
                    format!("Unknown binary magic: 0x{:08x}", magic),
                ))
            }
        }
    }
}

fn parse_fat_binary(bytes: &[u8], is_be: bool) -> UmrResult<MachOInspection> {
    let mut cursor = Cursor::new(bytes);
    let _magic = cursor.read_u32::<BigEndian>().map_err(read_err)?;
    let nfat_arch = if is_be {
        cursor.read_u32::<BigEndian>().map_err(read_err)? as usize
    } else {
        cursor.read_u32::<LittleEndian>().map_err(read_err)? as usize
    };

    let mut all_archs = Vec::new();
    let mut encryption = EncryptionStatus::Unencrypted;
    let mut is_simulator_build = false;

    for _ in 0..nfat_arch.min(16) {
        let (cputype, _cpusubtype, offset, _size) = if is_be {
            (
                cursor.read_u32::<BigEndian>().map_err(read_err)?,
                cursor.read_u32::<BigEndian>().map_err(read_err)?,
                cursor.read_u32::<BigEndian>().map_err(read_err)? as usize,
                cursor.read_u32::<BigEndian>().map_err(read_err)? as usize,
            )
        } else {
            (
                cursor.read_u32::<LittleEndian>().map_err(read_err)?,
                cursor.read_u32::<LittleEndian>().map_err(read_err)?,
                cursor.read_u32::<LittleEndian>().map_err(read_err)? as usize,
                cursor.read_u32::<LittleEndian>().map_err(read_err)? as usize,
            )
        };
        let _align = cursor.read_u32::<BigEndian>().map_err(read_err)?;

        let arch = cpu_type_to_arch(cputype);
        if !all_archs.contains(&arch) {
            all_archs.push(arch);
        }

        // Parse slice at offset
        if offset + 4 <= bytes.len() {
            let slice_magic = (&bytes[offset..offset + 4]).read_u32::<LittleEndian>().unwrap_or(0);
            let is_64 = slice_magic == MH_MAGIC_64 || slice_magic == MH_CIGAM_64;
            let is_le = slice_magic == MH_MAGIC || slice_magic == MH_MAGIC_64;
            if let Ok(slice_res) = parse_thin_macho(bytes, offset, is_64, is_le) {
                if let EncryptionStatus::FairPlayEncrypted { cryptid } = slice_res.encryption {
                    encryption = EncryptionStatus::FairPlayEncrypted { cryptid };
                }
                if slice_res.is_simulator_build {
                    is_simulator_build = true;
                }
            }
        }
    }

    Ok(MachOInspection {
        architectures: all_archs,
        encryption,
        is_simulator_build,
    })
}

fn parse_thin_macho(
    bytes: &[u8],
    offset: usize,
    is_64: bool,
    is_le: bool,
) -> UmrResult<MachOInspection> {
    if offset + 28 > bytes.len() {
        return Err(DiagnosticError::new(
            ErrorCode::ErrIntakeParseFailure,
            "Mach-O slice header truncated",
        ));
    }

    let mut cursor = Cursor::new(&bytes[offset..]);
    let _magic = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
    let cputype = if is_le {
        cursor.read_u32::<LittleEndian>().map_err(read_err)?
    } else {
        cursor.read_u32::<BigEndian>().map_err(read_err)?
    };
    let _cpusubtype = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
    let _filetype = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
    let ncmds = if is_le {
        cursor.read_u32::<LittleEndian>().map_err(read_err)? as usize
    } else {
        cursor.read_u32::<BigEndian>().map_err(read_err)? as usize
    };
    let _sizeofcmds = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
    let _flags = cursor.read_u32::<LittleEndian>().map_err(read_err)?;

    if is_64 {
        let _reserved = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
    }

    let arch = cpu_type_to_arch(cputype);
    let mut encryption = EncryptionStatus::Unencrypted;
    let mut is_simulator_build = arch == CpuArchitecture::X86_64; // x86_64 is always simulator for iOS

    for _ in 0..ncmds.min(500) {
        let cmd_start = cursor.position();
        let cmd = if is_le {
            cursor.read_u32::<LittleEndian>().map_err(read_err)?
        } else {
            cursor.read_u32::<BigEndian>().map_err(read_err)?
        };
        let cmdsize = if is_le {
            cursor.read_u32::<LittleEndian>().map_err(read_err)? as u64
        } else {
            cursor.read_u32::<BigEndian>().map_err(read_err)? as u64
        };

        if cmdsize < 8 {
            break;
        }

        match cmd {
            LC_ENCRYPTION_INFO | LC_ENCRYPTION_INFO_64 => {
                let _cryptoff = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
                let _cryptsize = cursor.read_u32::<LittleEndian>().map_err(read_err)?;
                let cryptid = if is_le {
                    cursor.read_u32::<LittleEndian>().map_err(read_err)?
                } else {
                    cursor.read_u32::<BigEndian>().map_err(read_err)?
                };

                if cryptid != 0 {
                    encryption = EncryptionStatus::FairPlayEncrypted { cryptid };
                }
            }
            LC_BUILD_VERSION => {
                let platform = if is_le {
                    cursor.read_u32::<LittleEndian>().map_err(read_err)?
                } else {
                    cursor.read_u32::<BigEndian>().map_err(read_err)?
                };
                if platform == PLATFORM_IOS_SIMULATOR {
                    is_simulator_build = true;
                }
            }
            _ => {}
        }

        let _ = cursor.set_position(cmd_start + cmdsize);
    }

    Ok(MachOInspection {
        architectures: vec![arch],
        encryption,
        is_simulator_build,
    })
}

fn cpu_type_to_arch(cputype: u32) -> CpuArchitecture {
    match cputype {
        CPU_TYPE_ARM => CpuArchitecture::Armv7,
        CPU_TYPE_ARM64 => CpuArchitecture::Arm64,
        CPU_TYPE_X86_64 => CpuArchitecture::X86_64,
        _ => CpuArchitecture::Unknown,
    }
}

fn read_err(e: std::io::Error) -> DiagnosticError {
    DiagnosticError::new(
        ErrorCode::ErrIntakeParseFailure,
        format!("Error parsing Mach-O binary: {}", e),
    )
}

