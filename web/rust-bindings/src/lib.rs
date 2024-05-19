use hades2::Result;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub fn expand_savefile(data: &[u8], format: &str) -> Result<String, JsError> {
    let savefile = hades2::parse(data)?;
    let lua_state = savefile.decompress_lua_state()?;
    let state = hades2::parse_lua_state(&lua_state)?;

    let text = match format {
        "text" => format!("{:#?}", state),
        "json" => serde_json::to_string(&state)?,
        "json-pretty" => serde_json::to_string_pretty(&state)?,
        _ => return Err(JsError::new("expected `json` or `text`")),
    };

    Ok(text)
}
