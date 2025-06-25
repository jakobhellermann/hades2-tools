use anyhow::{Result, anyhow};
use hades2::saves::Savefile;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let path_a = args
        .next()
        .ok_or_else(|| anyhow!("Expected path to file"))?;
    let path_b = args
        .next()
        .ok_or_else(|| anyhow!("Expected path to second file"))?;

    let data_a = std::fs::read(&path_a)?;
    let (_savefile_a, lua_a) = Savefile::parse(&data_a)?;

    let data_b = std::fs::read(&path_b)?;
    let (_savefile_b, lua_b) = Savefile::parse(&data_b)?;

    let tmp = std::env::temp_dir();
    let out_a = tmp.join("a.txt");
    std::fs::write(&out_a, format!("{lua_a:#?}"))?;
    let out_b = tmp.join("b.txt");
    std::fs::write(&out_b, format!("{lua_b:#?}"))?;

    println!("code --diff {} {}", out_a.display(), out_b.display());

    Ok(())
}
