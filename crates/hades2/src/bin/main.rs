use anyhow::Result;
use hades2::parse;
use std::path::Path;

fn main() -> Result<()> {
    let dir = Path::new("C:/Users/Jakob/Saved Games/Hades II");

    // let profiles = ["Profile1.sav", "Profile2.sav"];
    let profiles = ["Profile1.sav"];
    for profile in profiles {
        let profile = dir.join(profile);

        let bytes = std::fs::read(profile)?;
        parse(&mut &*bytes)?;
    }

    Ok(())
}
