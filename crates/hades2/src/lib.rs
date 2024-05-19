#![allow(unused)]

mod parser;
use parser::*;

pub use parser::luabins::Value as LuaValue;
pub use parser::Result;

pub struct Savefile<'a> {
    location: &'a str,
    lua_keys: Vec<&'a str>,
    lua_state_raw: &'a [u8],
}
impl<'a> Savefile<'a> {
    pub fn decompress_lua_state(&self) -> Result<Vec<u8>, lz4_flex::block::DecompressError> {
        lz4_flex::block::decompress(self.lua_state_raw, 2994108)
    }
}

pub fn parse_lua_state(mut lua_state: &[u8]) -> Result<LuaValue<'_>> {
    let lua_state = parser::luabins::read_luabins(&mut lua_state)?;
    if lua_state.len() != 1 {
        return Err(Error::LuaError);
    }

    let value = lua_state.into_iter().next().unwrap();

    Ok(value)
}

pub fn parse(mut data: &[u8]) -> Result<Savefile<'_>> {
    parse_inner(&mut data)
}
fn parse_inner<'i>(data: &mut &'i [u8]) -> Result<Savefile<'i>> {
    let signature = read_bytes_array::<4>(data)?;
    if signature != [0x53, 0x47, 0x42, 0x31] {
        return Err(Error::SignatureMismatch);
    }

    let _checksum = read_bytes_array::<4>(data);
    let version = read_u32(data)?;
    if version != 17 {
        return Err(Error::UnsupportedVersion(version));
    }

    let _a = read_u32(data)?;
    let _b = read_u32(data)?;

    let location = read_str_prefix(data)?;
    let unk_1 = read_u32(data)?;
    let unk_2 = read_u32(data)?; // runs?
    let unk_3 = read_u32(data)?; // meta points?
    let unk_4 = read_u32(data)?; // shrine points?
    let unk_5 = read_u8(data)?; // god mode?
    let unk_6 = read_u8(data)?; // hell mode?
                                // dbg!(unk_1, unk_2, unk_3, unk_4, unk_5, unk_6,);

    let lua_keys = read_array(data, read_str_prefix)?;

    let current_map_name = read_str_prefix(data)?;
    let start_next_map = read_str_prefix(data)?;

    let length = read_u32(data)?;
    let lua_state = read_bytes(data, length as usize)?;

    if data.len() > 0 {
        return Err(Error::UnexpectedAtEnd);
    }

    Ok(Savefile {
        location,
        lua_keys,
        lua_state_raw: lua_state,
    })
}
