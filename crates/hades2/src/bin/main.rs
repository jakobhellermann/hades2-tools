use anyhow::Result;
use hades2::Hades2Installation;

fn main() -> Result<()> {
    let hades = Hades2Installation::detect()?;

    for save in [2] {
        let save = hades.save(save)?;
        dbg!(&save);
        let _lua = save.parse_lua_state()?;
        // println!("{}...", &format!("{_lua:?}")[..100]);
    }

    Ok(())
}
