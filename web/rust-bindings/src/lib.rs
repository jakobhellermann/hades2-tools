use hades2::Result;
use wasm_bindgen::prelude::*;

fn expand_savefile_inner(data: &[u8]) -> Result<String> {
    let savefile = hades2::parse(data)?;
    let lua_state = savefile.decompress_lua_state()?;
    let state = hades2::parse_lua_state(&lua_state)?;

    Ok(format!("{:#?}", state))
}

#[wasm_bindgen]
pub fn expand_savefile(data: &[u8]) -> Result<String, String> {
    match expand_savefile_inner(data) {
        Ok(text) => Ok(text),
        Err(e) => Err(e.to_string()),
    }
}
