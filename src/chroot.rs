//! Chroot workspace creation and management.
//!
//! This module provides functionality to create isolated chroot environments
//! for workspace execution using debootstrap and Linux chroot syscall.

use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChrootError {
    #[error("Failed to create chroot directory: {0}")]
    DirectoryCreation(#[from] std::io::Error),

    #[error("Failed to remove chroot directory: {0}")]
    DirectoryRemoval(std::io::Error),

    #[error("Debootstrap failed: {0}")]
    Debootstrap(String),

    #[error("Pacstrap failed: {0}")]
    Pacstrap(String),

    #[error("Mount operation failed: {0}")]
    Mount(String),

    #[error("Chroot command failed: {0}")]
    ChrootExecution(String),

    #[error("Unsupported distribution: {0}")]
    UnsupportedDistro(String),
}

pub type ChrootResult<T> = Result<T, ChrootError>;

/// Supported Linux distributions for chroot environments
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChrootDistro {
    /// Ubuntu Noble (24.04 LTS)
    UbuntuNoble,
    /// Ubuntu Jammy (22.04 LTS)
    UbuntuJammy,
    /// Debian Bookworm (12)
    DebianBookworm,
    /// Arch Linux (base)
    ArchLinux,
}

impl ChrootDistro {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UbuntuNoble => "noble",
            Self::UbuntuJammy => "jammy",
            Self::DebianBookworm => "bookworm",
            Self::ArchLinux => "arch-linux",
        }
    }

    pub fn mirror_url(&self) -> &'static str {
        match self {
            Self::UbuntuNoble | Self::UbuntuJammy => "http://archive.ubuntu.com/ubuntu",
            Self::DebianBookworm => "http://deb.debian.org/debian",
            Self::ArchLinux => "https://geo.mirror.pkgbuild.com/",
        }
    }
}

impl Default for ChrootDistro {
    fn default() -> Self {
        Self::UbuntuNoble
    }
}

/// Create a minimal chroot environment using debootstrap or pacstrap
pub async fn create_chroot(
    chroot_path: &Path,
    distro: ChrootDistro,
) -> ChrootResult<()> {
    // Create the chroot directory
    tokio::fs::create_dir_all(chroot_path).await?;

    tracing::info!(
        "Creating chroot at {} with distro {}",
        chroot_path.display(),
        distro.as_str()
    );

    match distro {
        ChrootDistro::ArchLinux => create_arch_chroot(chroot_path).await?,
        _ => create_debootstrap_chroot(chroot_path, distro).await?,
    }

    tracing::info!("Chroot created successfully at {}", chroot_path.display());

    // Mount necessary filesystems
    mount_chroot_filesystems(chroot_path).await?;

    Ok(())
}

async fn create_debootstrap_chroot(
    chroot_path: &Path,
    distro: ChrootDistro,
) -> ChrootResult<()> {
    let output = tokio::process::Command::new("debootstrap")
        .arg("--variant=minbase")
        .arg(distro.as_str())
        .arg(chroot_path)
        .arg(distro.mirror_url())
        .output()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ChrootError::Debootstrap(
                    "debootstrap not found. Install debootstrap on the host.".to_string(),
                )
            } else {
                ChrootError::Debootstrap(e.to_string())
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ChrootError::Debootstrap(stderr.to_string()));
    }

    Ok(())
}

async fn create_arch_chroot(chroot_path: &Path) -> ChrootResult<()> {
    let pacman_conf = std::env::temp_dir().join("open_agent_pacman.conf");
    let pacman_conf_contents = r#"[options]
Architecture = auto
SigLevel = Never

[core]
Include = /etc/pacman.d/mirrorlist

[extra]
Include = /etc/pacman.d/mirrorlist
"#;
    tokio::fs::write(&pacman_conf, pacman_conf_contents).await?;

    let output = tokio::process::Command::new("pacstrap")
        .arg("-C")
        .arg(&pacman_conf)
        .arg("-c")
        .arg(chroot_path)
        .arg("base")
        .output()
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ChrootError::Pacstrap(
                    "pacstrap not found. Install arch-install-scripts (and pacman) on the host."
                        .to_string(),
                )
            } else {
                ChrootError::Pacstrap(e.to_string())
            }
        })?;

    let _ = tokio::fs::remove_file(&pacman_conf).await;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ChrootError::Pacstrap(stderr.to_string()));
    }

    Ok(())
}

/// Mount necessary filesystems for chroot environment
async fn mount_chroot_filesystems(chroot_path: &Path) -> ChrootResult<()> {
    let mounts = vec![
        ("proc", "proc", "/proc"),
        ("sysfs", "sysfs", "/sys"),
        ("devpts", "devpts", "/dev/pts"),
        ("tmpfs", "tmpfs", "/dev/shm"),
    ];

    for (fstype, source, target) in mounts {
        let mount_point = chroot_path.join(target.trim_start_matches('/'));
        tokio::fs::create_dir_all(&mount_point).await?;

        let output = tokio::process::Command::new("mount")
            .arg("-t")
            .arg(fstype)
            .arg(source)
            .arg(&mount_point)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't fail if mount is already mounted
            if !stderr.contains("already mounted") {
                return Err(ChrootError::Mount(stderr.to_string()));
            }
        }

        tracing::debug!("Mounted {} at {}", fstype, mount_point.display());
    }

    Ok(())
}

/// Unmount filesystems from chroot environment
pub async fn unmount_chroot_filesystems(chroot_path: &Path) -> ChrootResult<()> {
    if !chroot_path.exists() {
        tracing::info!(
            "Chroot path {} does not exist, skipping unmount",
            chroot_path.display()
        );
        return Ok(());
    }

    let targets = vec!["/dev/shm", "/dev/pts", "/sys", "/proc"];

    for target in targets {
        let mount_point = chroot_path.join(target.trim_start_matches('/'));

        let output = tokio::process::Command::new("umount")
            .arg(&mount_point)
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't fail if not mounted
            if !stderr.contains("not mounted") {
                tracing::warn!("Failed to unmount {}: {}", mount_point.display(), stderr);
            }
        }
    }

    Ok(())
}

/// Execute a command inside a chroot environment
pub async fn execute_in_chroot(
    chroot_path: &Path,
    command: &[String],
) -> ChrootResult<std::process::Output> {
    if command.is_empty() {
        return Err(ChrootError::ChrootExecution(
            "Empty command".to_string(),
        ));
    }

    // Build the chroot command
    let output = tokio::process::Command::new("chroot")
        .arg(chroot_path)
        .args(command)
        .output()
        .await?;

    Ok(output)
}

/// Check if a chroot environment is already created and fully functional.
/// This checks both essential directories and required mount points.
pub async fn is_chroot_created(chroot_path: &Path) -> bool {
    // Check for essential directories that indicate debootstrap completed
    let essential_paths = vec!["bin", "usr", "etc", "var"];

    for path in essential_paths {
        let full_path = chroot_path.join(path);
        if !full_path.exists() {
            return false;
        }
    }

    // Also check that mount points exist and are mounted
    // This ensures the chroot is fully initialized, not just partially created
    let mount_points = vec!["proc", "sys", "dev/pts", "dev/shm"];
    for mount in mount_points {
        let mount_path = chroot_path.join(mount);
        if !mount_path.exists() {
            return false;
        }
    }

    // Verify /proc is actually mounted by checking for /proc/1 (init process)
    let proc_check = chroot_path.join("proc/1");
    if !proc_check.exists() {
        return false;
    }

    true
}

fn parse_os_release_value(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{}=", key);
    if !line.starts_with(&prefix) {
        return None;
    }
    let value = line[prefix.len()..].trim().trim_matches('"');
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// Detect the distro of an existing chroot by inspecting /etc/os-release.
pub async fn detect_chroot_distro(chroot_path: &Path) -> Option<ChrootDistro> {
    let os_release_path = chroot_path.join("etc/os-release");
    let contents = tokio::fs::read_to_string(os_release_path).await.ok()?;
    let mut id: Option<String> = None;
    let mut codename: Option<String> = None;

    for line in contents.lines() {
        if id.is_none() {
            id = parse_os_release_value(line, "ID");
        }
        if codename.is_none() {
            codename = parse_os_release_value(line, "VERSION_CODENAME");
        }
    }

    match id.as_deref()? {
        "ubuntu" => match codename.as_deref()? {
            "noble" => Some(ChrootDistro::UbuntuNoble),
            "jammy" => Some(ChrootDistro::UbuntuJammy),
            _ => None,
        },
        "debian" => match codename.as_deref()? {
            "bookworm" => Some(ChrootDistro::DebianBookworm),
            _ => None,
        },
        "arch" | "archlinux" => Some(ChrootDistro::ArchLinux),
        _ => None,
    }
}

/// Clean up a chroot environment
pub async fn destroy_chroot(chroot_path: &Path) -> ChrootResult<()> {
    tracing::info!("Destroying chroot at {}", chroot_path.display());

    if !chroot_path.exists() {
        tracing::info!(
            "Chroot path {} does not exist, nothing to destroy",
            chroot_path.display()
        );
        return Ok(());
    }

    // Unmount filesystems first
    unmount_chroot_filesystems(chroot_path).await?;

    // Remove the chroot directory
    match tokio::fs::remove_dir_all(chroot_path).await {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(ChrootError::DirectoryRemoval(e)),
    }

    tracing::info!("Chroot destroyed successfully");

    Ok(())
}
