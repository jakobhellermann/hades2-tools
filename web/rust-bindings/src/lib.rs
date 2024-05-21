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
    let (_savefile, lua_state) = hades2::saves::Savefile::parse(data)?;

    let text = match format {
        "text" => format!("{:#?}", lua_state),
        "json" => serde_json::to_string(&lua_state)?,
        "json-pretty" => serde_json::to_string_pretty(&lua_state)?,
        _ => return Err(JsError::new("expected `json` or `text`")),
    };

    Ok(text)
}
