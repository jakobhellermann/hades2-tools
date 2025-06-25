use anyhow::{Result, anyhow};
use hades2::saves::Savefile;

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow!("Expected path to file as first argument"))?;

    let data = std::fs::read(&path)?;
    let (mut savefile, lua) = Savefile::parse(&data)?;
    dbg!(&savefile);

    let mut out = Vec::new();
    savefile.serialize(&mut out, &lua)?;

    let (mut savefile_reparsed, lua_reparsed) = Savefile::parse(&out)?;

    savefile.checksum = 0;
    savefile_reparsed.checksum = 0;

    assert_eq!(lua, lua_reparsed);
    assert_eq!(savefile_reparsed, savefile);

    Ok(())
}
