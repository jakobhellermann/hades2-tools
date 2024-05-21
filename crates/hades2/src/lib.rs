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

    pub fn saves(&self) -> Result<Vec<SaveHandle>> {
        let mut saves = Vec::new();
        for save in self.save_dir.read_dir()? {
            if let Some(handle) = SaveHandle::from_path(save?.path()) {
                saves.push(handle);
            }
        }

        Ok(saves)
    }
}

#[derive(Clone, Debug)]
pub struct SaveHandle(PathBuf, u32);
impl SaveHandle {
    pub fn from_path(path: PathBuf) -> Option<Self> {
        let i = path
            .file_name()?
            .to_str()?
            .strip_suffix(".sav")?
            .strip_prefix("Profile")?
            .parse()
            .ok()?;

        Some(SaveHandle(path, i))
    }

    pub fn slot(&self) -> u32 {
        self.1
    }
    pub fn path(&self) -> &Path {
        &self.0
    }
    pub fn read(&self) -> Result<(saves::Savefile, LuaValue<'static>)> {
        let data = std::fs::read(&self.0)?;
        let result = saves::Savefile::parse(&data)?;
        Ok(result)
    }

    pub fn read_header_only(&self) -> Result<saves::Savefile> {
        let data = std::fs::read(&self.0)?;
        let savefile = saves::Savefile::parse_header_only(&data)?;
        Ok(savefile)
    }
}
