mod parser;
mod steamlocate;

pub mod saves;

use anyhow::Context;
pub use anyhow::{Error, Result};
use saves::LuaValue;
use std::path::{Path, PathBuf};

#[allow(unused)]
#[derive(Debug, thiserror::Error)]
pub(crate) enum LocateError {
    #[error("platform not supported")]
    UnsupportedPlatform,
    #[error("directory was not found")]
    NotFound,
    #[error("error trying to find directory")]
    Other,
}

#[derive(Debug, Clone)]
pub struct Hades2Installation {
    #[allow(unused)]
    steam_dir: PathBuf,
    save_dir: PathBuf,
}
impl Hades2Installation {
    pub fn steam_dir(&self) -> &Path {
        &self.steam_dir
    }
    pub fn save_dir(&self) -> &Path {
        &self.save_dir
    }

    pub fn detect() -> Result<Self> {
        let steam_dir = crate::steamlocate::locate_steam_dir()?;
        let save_dir = saves::save_dir(&steam_dir)?;

        Ok(Hades2Installation {
            steam_dir,
            save_dir,
        })
    }

    pub fn save(&self, slot: u32) -> Result<SaveHandle> {
        let path = self.save_dir.join(format!("Profile{slot}.sav"));
        anyhow::ensure!(path.exists(), "save {slot} does not exist");
        let handle = SaveHandle::from_path(path).context("context")?;
        Ok(handle)
    }
    pub fn backups(&self, slot: u32) -> Result<Vec<SaveHandle>> {
        let mut saves =
            self.saves_inner(|handle| handle.slot == slot && handle.backup_index.is_some())?;
        saves.sort_by_key(|handle| std::cmp::Reverse(handle.backup_index));
        Ok(saves)
    }

    pub fn saves(&self) -> Result<Vec<SaveHandle>> {
        let mut saves = self.saves_inner(|handle| handle.backup_index().is_none())?;
        saves.sort_by_key(|handle| (handle.slot, handle.backup_index));
        Ok(saves)
    }

    fn saves_inner(&self, f: impl Fn(&SaveHandle) -> bool) -> Result<Vec<SaveHandle>> {
        let mut saves = Vec::new();
        for save in self.save_dir.read_dir()? {
            if let Some(handle) = SaveHandle::from_path(save?.path()) {
                if f(&handle) {
                    saves.push(handle);
                }
            }
        }

        Ok(saves)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveHandle {
    path: PathBuf,
    slot: u32,
    backup_index: Option<u32>,
}
impl SaveHandle {
    pub fn from_path(path: PathBuf) -> Option<Self> {
        let stem = path.file_stem()?.to_str()?;
        let ext = path.extension()?.to_str()?;

        let bak = ext.strip_prefix("bak").and_then(|x| x.parse::<u32>().ok());

        let stem = match bak {
            Some(_) => stem.strip_suffix(".sav")?,
            None if ext != "sav" => return None,
            None => stem,
        };

        let slot = stem.strip_prefix("Profile")?.parse().ok()?;

        Some(SaveHandle {
            path,
            slot,
            backup_index: bak,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn slot(&self) -> u32 {
        self.slot
    }
    pub fn backup_index(&self) -> Option<u32> {
        self.backup_index
    }

    pub fn read(&self) -> Result<(saves::Savefile, LuaValue<'static>)> {
        let data = std::fs::read(&self.path)?;
        let result = saves::Savefile::parse(&data)?;
        Ok(result)
    }

    pub fn read_header_only(&self) -> Result<saves::Savefile> {
        let data = std::fs::read(&self.path)?;
        let savefile = saves::Savefile::parse_header_only(&data)?;
        Ok(savefile)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::SaveHandle;

    #[test]
    fn save_handle_ok() {
        let path = PathBuf::from("Profile1.sav");
        assert_eq!(
            SaveHandle::from_path(path.clone()).unwrap(),
            SaveHandle {
                path: path,
                slot: 1,
                backup_index: None
            }
        );

        let path = PathBuf::from("Profile10.sav");
        assert_eq!(
            SaveHandle::from_path(path.clone()).unwrap(),
            SaveHandle {
                path: path,
                slot: 10,
                backup_index: None
            }
        );
    }

    #[test]
    fn save_handle_invalid_ext() {
        let path = PathBuf::from("Profile1.sjson");
        assert_eq!(SaveHandle::from_path(path.clone()), None);
    }

    #[test]
    fn save_handle_bak() {
        let path = PathBuf::from("Profile1.sav.bak1");
        assert_eq!(
            SaveHandle::from_path(path.clone()).unwrap(),
            SaveHandle {
                path: path,
                slot: 1,
                backup_index: Some(1)
            }
        );

        let path = PathBuf::from("Profile2.sav.bak10");
        assert_eq!(
            SaveHandle::from_path(path.clone()).unwrap(),
            SaveHandle {
                path: path,
                slot: 2,
                backup_index: Some(10)
            }
        );
    }
}
