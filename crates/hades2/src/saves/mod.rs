use crate::LocateError;
use std::path::{Path, PathBuf};

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub(crate) fn save_dir(_steam_dir: &Path) -> Result<PathBuf, LocateError> {
    Err(LocateError::UnsupportedPlatform)
}

#[cfg(target_os = "windows")]
pub(crate) fn save_dir(_steam_dir: &Path) -> Result<PathBuf, LocateError> {
    let home_dir = dirs::home_dir().ok_or_else(|| LocateError::Other)?;
    let dir = home_dir.join("Saved Games/Hades II");
    if !dir.exists() {
        return Err(LocateError::NotFound);
    }
    Ok(dir)
}

#[cfg(target_os = "macos")]
pub(crate) fn save_dir(_steam_dir: &Path) -> Result<PathBuf, LocateError> {
    todo!()
}

#[cfg(target_os = "linux")]
pub(crate) fn save_dir(steam_dir: &Path) -> Result<PathBuf, LocateError> {
    // if !dir.exists() {
    // // return Err(Error::NotFound);
    // }

    let dir = steam_dir
        .join("steamapps/compatdata/1145350/pfx/drive_c/users/steamuser/Saved Games/Hades II");
    if !dir.exists() {
        return Err(LocateError::NotFound);
    }

    Ok(dir)
}

pub use crate::parser::luabins::Value as LuaValue;
pub use crate::parser::Result;
use crate::parser::*;

#[derive(Clone, Debug)]
pub struct Savefile {
    pub location: String,
    pub checksum: u32,
    pub timestamp: u64,
    pub runs: u32,
    pub accumulated_meta_points: u32,
    pub active_shrine_points: u32,
    pub grasp: u32,
    pub easy_mode: bool,
    pub hard_mode: bool,
    pub lua_keys: Vec<String>,
    pub current_map_name: String,
    pub start_next_map: String,
}

impl Savefile {
    pub fn parse(mut data: &[u8]) -> Result<(Savefile, LuaValue<'static>)> {
        let computed_checksum = adler32::RollingAdler32::from_buffer(&data[8..]).hash();

        let (savefile, lua_state) = parse_inner(&mut data)?;
        let lua_state = lz4_flex::block::decompress(lua_state, 15679488)?;

        let lua_state = luabins::read_luabins(&mut lua_state.as_slice())?;
        if lua_state.len() != 1 {
            return Err(Error::LuaError);
        }
        let lua_state = lua_state.into_iter().next().unwrap();

        if computed_checksum != savefile.checksum {
            return Err(Error::ChecksumError);
        }

        Ok((savefile, lua_state))
    }

    pub fn parse_header_only(mut data: &[u8]) -> Result<Savefile> {
        let (savefile, _) = parse_inner(&mut data)?;
        Ok(savefile)
    }
}

fn parse_inner<'i>(data: &mut &'i [u8]) -> Result<(Savefile, &'i [u8])> {
    let signature = read_bytes_array::<4>(data)?;
    if signature != [0x53, 0x47, 0x42, 0x31] {
        return Err(Error::SignatureMismatch);
    }

    let checksum = read_u32(data)?;

    let version = read_u32(data)?;
    if version != 17 {
        return Err(Error::UnsupportedVersion(version));
    }
    let timestamp = read_u64(data)?;
    let location = read_str_prefix(data)?;
    let runs = read_u32(data)? + 1;
    let accumulated_meta_points = read_u32(data)?;
    let active_shrine_points = read_u32(data)?;
    let grasp = read_u32(data)?;
    let easy_mode = read_bool(data)?;
    let hard_mode = read_bool(data)?;

    let lua_keys = read_array(data, |data| read_str_prefix(data).map(ToOwned::to_owned))?;

    let current_map_name = read_str_prefix(data)?;
    let start_next_map = read_str_prefix(data)?;

    let length = read_u32(data)?;

    let lua_state = read_bytes(data, length as usize)?;

    if data.len() > 0 {
        return Err(Error::UnexpectedAtEnd);
    }

    Ok((
        Savefile {
            checksum,
            location: location.to_owned(),
            timestamp,
            runs,
            accumulated_meta_points,
            active_shrine_points,
            grasp,
            easy_mode,
            hard_mode,
            lua_keys,
            current_map_name: current_map_name.to_owned(),
            start_next_map: start_next_map.to_owned(),
        },
        lua_state,
    ))
}
