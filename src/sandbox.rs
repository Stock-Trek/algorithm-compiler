use crate::error::{ACError, ACResult};
use std::{
    env, fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};
use tokio::process::Command;

const UNTRUSTED_BUILD_UID_ENV: &str = "UNTRUSTED_BUILD_UID";
const UNTRUSTED_BUILD_GID_ENV: &str = "UNTRUSTED_BUILD_GID";
const UNTRUSTED_CARGO_HOME_ENV: &str = "UNTRUSTED_CARGO_HOME";
const CARGO_HOME_ENV: &str = "CARGO_HOME";
const DEFAULT_CARGO_HOME: &str = "/usr/local/cargo";

pub struct Sandbox;

impl Sandbox {
    pub fn configure(command: &mut Command) -> ACResult<()> {
        if let Some(cargo_home) = env::var_os(UNTRUSTED_CARGO_HOME_ENV) {
            command.env(CARGO_HOME_ENV, cargo_home);
        }
        if let Some((uid, gid)) = Self::ids()? {
            use std::os::unix::process::CommandExt;
            command.as_std_mut().uid(uid).gid(gid);
        }
        Ok(())
    }

    pub async fn prepare_cargo_home() -> ACResult<()> {
        tokio::task::spawn_blocking(Self::prepare_cargo_home_blocking)
            .await
            .map_err(|_| ACError::InternalServer("Sandbox cargo home task panicked".into()))?
    }

    pub async fn chown(path: &Path) -> ACResult<()> {
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || Self::chown_blocking(&path))
            .await
            .map_err(|_| ACError::InternalServer("Sandbox chown task panicked".into()))?
    }

    pub async fn chown_recursive(path: &Path) -> ACResult<()> {
        let path = path.to_path_buf();
        tokio::task::spawn_blocking(move || Self::chown_recursive_blocking(&path))
            .await
            .map_err(|_| ACError::InternalServer("Sandbox chown task panicked".into()))?
    }

    fn prepare_cargo_home_blocking() -> ACResult<()> {
        let Some(untrusted) = env::var_os(UNTRUSTED_CARGO_HOME_ENV) else {
            return Ok(());
        };
        let untrusted = PathBuf::from(untrusted);
        let trusted = env::var_os(CARGO_HOME_ENV)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_CARGO_HOME));
        Self::remove_dir_all_if_exists(&untrusted)?;
        fs::create_dir_all(&untrusted).map_err(ACError::FileSystem)?;
        for entry in ["registry", "config.toml", "config"] {
            let source = trusted.join(entry);
            if source.exists() {
                std::os::unix::fs::symlink(&source, untrusted.join(entry))
                    .map_err(ACError::FileSystem)?;
            }
        }
        let Some((uid, gid)) = Self::ids()? else {
            return Ok(());
        };
        Self::chown_tree(&untrusted, uid, gid)
    }

    fn chown_blocking(path: &Path) -> ACResult<()> {
        let Some((uid, gid)) = Self::ids()? else {
            return Ok(());
        };
        match std::os::unix::fs::lchown(path, Some(uid), Some(gid)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(ACError::FileSystem(error)),
        }
    }

    fn chown_recursive_blocking(path: &Path) -> ACResult<()> {
        let Some((uid, gid)) = Self::ids()? else {
            return Ok(());
        };
        Self::chown_tree(path, uid, gid)
    }

    fn chown_tree(path: &Path, uid: u32, gid: u32) -> ACResult<()> {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(ACError::FileSystem(error)),
        };
        if metadata.uid() == uid && metadata.gid() == gid {
            return Ok(());
        }
        std::os::unix::fs::lchown(path, Some(uid), Some(gid)).map_err(ACError::FileSystem)?;
        if !metadata.is_dir() {
            return Ok(());
        }
        for entry in fs::read_dir(path).map_err(ACError::FileSystem)? {
            let entry = entry.map_err(ACError::FileSystem)?;
            Self::chown_tree(&entry.path(), uid, gid)?;
        }
        Ok(())
    }

    fn remove_dir_all_if_exists(path: &Path) -> ACResult<()> {
        match fs::remove_dir_all(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(ACError::FileSystem(error)),
        }
    }

    fn ids() -> ACResult<Option<(u32, u32)>> {
        match (
            Self::id(UNTRUSTED_BUILD_UID_ENV)?,
            Self::id(UNTRUSTED_BUILD_GID_ENV)?,
        ) {
            (Some(uid), Some(gid)) => Ok(Some((uid, gid))),
            (None, None) => Ok(None),
            _ => Err(ACError::Config(format!(
                "{UNTRUSTED_BUILD_UID_ENV} and {UNTRUSTED_BUILD_GID_ENV} must be set together"
            ))),
        }
    }

    fn id(key: &str) -> ACResult<Option<u32>> {
        match env::var(key) {
            Ok(value) => value
                .parse::<u32>()
                .map(Some)
                .map_err(|_| ACError::Config(format!("{key} must be a valid id"))),
            Err(env::VarError::NotPresent) => Ok(None),
            Err(env::VarError::NotUnicode(_)) => {
                Err(ACError::Config(format!("{key} is not valid UTF-8")))
            }
        }
    }
}
