use std::fs;
use std::path::{Path, PathBuf};

use umr_core::{DiagnosticError, ErrorCode, UmrResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemImageInfo {
    pub api_level: u32,
    pub tag: String,
    pub abi: String,
    pub path: PathBuf,
}

pub struct AvdProvisioner;

impl AvdProvisioner {
    /// Discovers installed Android SDK system images under `system-images/`
    pub fn discover_system_images(sdk_root: &Path) -> Vec<SystemImageInfo> {
        let mut results = Vec::new();
        let sys_images_root = sdk_root.join("system-images");
        if !sys_images_root.exists() {
            return results;
        }

        // Hierarchy is typically: system-images/android-<api>/<tag>/<abi>/
        if let Ok(api_entries) = fs::read_dir(&sys_images_root) {
            for api_entry in api_entries.flatten() {
                let api_path = api_entry.path();
                if !api_path.is_dir() {
                    continue;
                }
                let api_folder_name = api_entry.file_name().to_string_lossy().to_string();
                let api_level = api_folder_name
                    .strip_prefix("android-")
                    .and_then(|v| v.split('.').next())
                    .and_then(|v| v.parse::<u32>().ok())
                    .unwrap_or(0);

                if let Ok(tag_entries) = fs::read_dir(&api_path) {
                    for tag_entry in tag_entries.flatten() {
                        let tag_path = tag_entry.path();
                        if !tag_path.is_dir() {
                            continue;
                        }
                        let tag_name = tag_entry.file_name().to_string_lossy().to_string();

                        if let Ok(abi_entries) = fs::read_dir(&tag_path) {
                            for abi_entry in abi_entries.flatten() {
                                let abi_path = abi_entry.path();
                                if !abi_path.is_dir() {
                                    continue;
                                }
                                let abi_name = abi_entry.file_name().to_string_lossy().to_string();

                                // Verify kernel or system.img exists
                                if abi_path.join("system.img").exists() || abi_path.join("kernel-ranchu").exists() {
                                    results.push(SystemImageInfo {
                                        api_level,
                                        tag: tag_name.clone(),
                                        abi: abi_name,
                                        path: abi_path,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Sort descending by API level (prefer newest installed system image)
        results.sort_by(|a, b| b.api_level.cmp(&a.api_level));
        results
    }

    /// Resolves the canonical user AVD directory (`~/.android/avd/`)
    pub fn get_avd_root() -> UmrResult<PathBuf> {
        if let Ok(avd_home) = std::env::var("ANDROID_AVD_HOME") {
            let p = PathBuf::from(avd_home);
            if !p.exists() {
                let _ = fs::create_dir_all(&p);
            }
            return Ok(p);
        }

        let home = if cfg!(windows) {
            std::env::var("USERPROFILE")
                .or_else(|_| std::env::var("HOME"))
                .map(PathBuf::from)
        } else {
            std::env::var("HOME").map(PathBuf::from)
        };

        match home {
            Ok(h) => {
                let avd_dir = h.join(".android").join("avd");
                if !avd_dir.exists() {
                    let _ = fs::create_dir_all(&avd_dir);
                }
                Ok(avd_dir)
            }
            Err(_) => Err(DiagnosticError::new(
                ErrorCode::ErrBackendInternal,
                "Failed to resolve user home directory for AVD configuration",
            )),
        }
    }

    /// Automatically provisions a dedicated hardware-accelerated AVD if not already present.
    pub fn provision_avd_if_missing(
        avd_name: &str,
        system_image: &SystemImageInfo,
        sdk_root: &Path,
    ) -> UmrResult<PathBuf> {
        let avd_root = Self::get_avd_root()?;
        let avd_ini_file = avd_root.join(format!("{}.ini", avd_name));
        let avd_dir = avd_root.join(format!("{}.avd", avd_name));

        // If both already exist, provision is complete
        if avd_ini_file.exists() && avd_dir.join("config.ini").exists() {
            return Ok(avd_dir);
        }

        if !avd_dir.exists() {
            fs::create_dir_all(&avd_dir).map_err(|e| {
                DiagnosticError::new(
                    ErrorCode::ErrBackendInternal,
                    format!("Failed to create AVD directory: {}", e),
                )
            })?;
        }

        // 1. Write <avd_name>.ini
        let ini_content = format!(
            "avd.ini.encoding=UTF-8\r\npath={}\r\npath.rel=avd/{}.avd\r\ntarget=android-{}\r\n",
            avd_dir.display(),
            avd_name,
            system_image.api_level
        );
        fs::write(&avd_ini_file, ini_content).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendInternal,
                format!("Failed to write AVD entry ini: {}", e),
            )
        })?;

        // 2. Relative or absolute image sysdir
        // Relative to sdk root: system-images/android-<api>/<tag>/<abi>/
        let relative_sysdir = match system_image.path.strip_prefix(sdk_root) {
            Ok(rel) => rel.to_string_lossy().replace('\\', "/"),
            Err(_) => system_image.path.to_string_lossy().replace('\\', "/"),
        };
        let formatted_sysdir = if relative_sysdir.ends_with('/') {
            relative_sysdir
        } else {
            format!("{}/", relative_sysdir)
        };

        // 3. Write <avd_name>.avd/config.ini
        let config_content = format!(
            "PlayStore.enabled=true\r\n\
             abi.type={abi}\r\n\
             avd.ini.displayname=UMR Autonomous Guest ({abi})\r\n\
             hw.cpu.arch={abi}\r\n\
             hw.cpu.ncore=4\r\n\
             hw.device.manufacturer=Google\r\n\
             hw.device.name=pixel_7\r\n\
             hw.gpu.enabled=yes\r\n\
             hw.gpu.mode=auto\r\n\
             hw.initialOrientation=Portrait\r\n\
             hw.keyboard=yes\r\n\
             hw.lcd.density=420\r\n\
             hw.lcd.height=2400\r\n\
             hw.lcd.width=1080\r\n\
             hw.ramSize=2048\r\n\
             hw.sdCard=yes\r\n\
             image.sysdir.1={sysdir}\r\n\
             tag.display={tag}\r\n\
             tag.id={tag}\r\n\
             vm.heapSize=256\r\n",
            abi = system_image.abi,
            sysdir = formatted_sysdir,
            tag = system_image.tag
        );

        fs::write(avd_dir.join("config.ini"), config_content).map_err(|e| {
            DiagnosticError::new(
                ErrorCode::ErrBackendInternal,
                format!("Failed to write AVD config.ini: {}", e),
            )
        })?;

        Ok(avd_dir)
    }
}

