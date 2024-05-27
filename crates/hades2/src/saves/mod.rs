use crate::LocateError;
use std::path::{Path, PathBuf};

const MAGIC: [u8; 4] = [0x53, 0x47, 0x42, 0x31];
const LZ4_MIN_DECOPMRESS_LEN: usize = 15679488;

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub(crate) fn save_dir(_steam_dir: &Path) -> Result<PathBuf, LocateError> {
    Err(LocateError::UnsupportedPlatform)
}

#[cfg(target_os = "windows")]
pub(crate) fn save_dir(_steam_dir: &Path) -> Result<PathBuf, LocateError> {
    let home_dir = dirs::home_dir().ok_or_else(|| LocateError::Other)?;
    let dir = home_dir.join("Saved Games/Hades II");
    if !dir.exists() {
        return Err(LocateError::NotFound("`~/Saved Games/Hades II`"));
    }
    Ok(dir)
}

#[cfg(target_os = "macos")]
pub(crate) fn save_dir(_steam_dir: &Path) -> Result<PathBuf, LocateError> {
    todo!()
}

#[cfg(target_os = "linux")]
pub(crate) fn save_dir(steam_dir: &Path) -> Result<PathBuf, LocateError> {
    let dir = steam_dir
        .join("steamapps/compatdata/1145350/pfx/drive_c/users/steamuser/Saved Games/Hades II");
    if !dir.exists() {
        return Err(LocateError::NotFound("steam game 1145350"));
    }

    Ok(dir)
}

pub use crate::parser::luabins::{LuaTable, Value as LuaValue};
pub use crate::parser::Result;
use crate::parser::*;

#[derive(Clone, Debug, PartialEq, Eq)]
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
        let lua_state = lz4_flex::block::decompress(lua_state, LZ4_MIN_DECOPMRESS_LEN)?;

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

pub(crate) fn parse_active_profile<'i>(data: &mut &'i [u8]) -> Result<&'i str> {
    let signature = read_bytes_array::<4>(data)?;
    if signature != MAGIC {
        return Err(Error::SignatureMismatch);
    }

    let str = read_str_prefix(data)?;

    if data.len() > 0 {
        return Err(Error::UnexpectedAtEnd);
    }

    Ok(str)
}

fn parse_inner<'i>(data: &mut &'i [u8]) -> Result<(Savefile, &'i [u8])> {
    let signature = read_bytes_array::<4>(data)?;
    if signature != MAGIC {
        return Err(Error::SignatureMismatch);
    }

    let checksum = read_u32(data)?;

    let version = read_u32(data)?;
    if version != 17 {
        return Err(Error::UnsupportedVersion(version));
    }
    let timestamp = read_u64(data)?;
    let location = read_str_prefix(data)?;
    let runs = read_u32(data)?;
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

impl Savefile {
    pub fn serialize<W: std::io::Write>(
        &self,
        out: W,
        lua_state: &LuaValue<'_>,
    ) -> std::io::Result<()> {
        let mut lua_state_bytes = Vec::new();
        luabins::write_luabins(&mut lua_state_bytes, std::iter::once(lua_state));

        let compressed = lz4_flex::compress(&lua_state_bytes);
        serialize_inner(out, self, &compressed)
    }
}

fn serialize_inner<W: std::io::Write>(
    mut out: W,
    savefile: &Savefile,
    lua_state_compressed: &[u8],
) -> std::io::Result<()> {
    let mut header = Vec::new();

    let version = 17;
    header.extend_from_slice(&u32::to_le_bytes(version));
    header.extend_from_slice(&u64::to_le_bytes(savefile.timestamp));
    header.extend_from_slice(&u32::to_le_bytes(savefile.location.len() as u32));
    header.extend_from_slice(savefile.location.as_bytes());
    header.extend_from_slice(&u32::to_le_bytes(savefile.runs));
    header.extend_from_slice(&u32::to_le_bytes(savefile.accumulated_meta_points));
    header.extend_from_slice(&u32::to_le_bytes(savefile.active_shrine_points));
    header.extend_from_slice(&u32::to_le_bytes(savefile.grasp));
    header.push(savefile.easy_mode as u8);
    header.push(savefile.hard_mode as u8);
    header.extend_from_slice(&u32::to_le_bytes(savefile.lua_keys.len() as u32));
    for key in &savefile.lua_keys {
        header.extend_from_slice(&u32::to_le_bytes(key.len() as u32));
        header.extend_from_slice(key.as_bytes());
    }
    header.extend_from_slice(&u32::to_le_bytes(savefile.current_map_name.len() as u32));
    header.extend_from_slice(savefile.current_map_name.as_bytes());
    header.extend_from_slice(&u32::to_le_bytes(savefile.start_next_map.len() as u32));
    header.extend_from_slice(savefile.start_next_map.as_bytes());
    header.extend_from_slice(&u32::to_le_bytes(lua_state_compressed.len() as u32));

    let mut checksum = adler32::RollingAdler32::from_buffer(&header);
    checksum.update_buffer(lua_state_compressed);
    let checksum = checksum.hash();

    out.write_all(&MAGIC)?;
    out.write_all(&u32::to_le_bytes(checksum))?;
    out.write_all(&header)?;
    out.write_all(lua_state_compressed)?;

    Ok(())
}

#[cfg(test)]
#[allow(const_item_mutation)]
mod test {
    use anyhow::Result;

    use super::LZ4_MIN_DECOPMRESS_LEN;

    // const EXAMPLE_PROFILE:&[u8] = include_bytes!("C:/Users/Jakob/Saved Games/Hades II/Profile1.sav").as_slice();
    const EXAMPLE_PROFILE: &[u8] = include_bytes!("/home/jakob/.local/share/Steam/steamapps/compatdata/1145350/pfx/drive_c/users/steamuser/Saved Games/Hades II/Profile1.sav").as_slice();

    #[test]
    fn roundtrip_luabins() -> Result<()> {
        let (_, lua_state) = super::parse_inner(&mut EXAMPLE_PROFILE)?;
        let lua_state_bytes = lz4_flex::block::decompress(lua_state, LZ4_MIN_DECOPMRESS_LEN)?;
        let lua_state = super::luabins::read_luabins(&mut lua_state_bytes.as_slice())?;

        /*for val in &lua_state {
            val.visit(true, &mut |val| {
                let mut result = Vec::new();
                super::luabins::write::save_value(&mut result, val);
                let reparsed = super::luabins::read_value(&mut result.as_slice()).unwrap();
                assert_eq!(*val, reparsed);
            });
        }*/

        let mut lua_state_bytes_again = Vec::new();
        super::luabins::write_luabins(&mut lua_state_bytes_again, lua_state.iter());
        let reparsed = super::luabins::read_luabins(&mut lua_state_bytes_again.as_slice()).unwrap();
        assert_eq!(*lua_state, reparsed);

        Ok(())
    }

    #[test]
    fn roundtrip_savefile() -> Result<()> {
        let (savefile, lua_state_compressed) = super::parse_inner(&mut EXAMPLE_PROFILE)?;

        let mut out = Vec::new();
        super::serialize_inner(&mut out, &savefile, lua_state_compressed)?;

        assert!(out.as_slice() == EXAMPLE_PROFILE);

        Ok(())
    }

    #[test]
    fn roundtrip_reparse_savefile() -> Result<()> {
        let (mut savefile, lua_state) = super::Savefile::parse(EXAMPLE_PROFILE)?;

        let mut out = Vec::new();
        savefile.serialize(&mut out, &lua_state)?;

        let (mut savefile_reparsed, lua_state_reparsed) = super::Savefile::parse(&out)?;

        savefile.checksum = 0;
        savefile_reparsed.checksum = 0;

        assert_eq!(savefile, savefile_reparsed);
        assert_eq!(lua_state, lua_state_reparsed);

        Ok(())
    }
}
