pub mod luabins;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("File does not begin with correct signature")]
    SignatureMismatch,
    #[error("Savefile version is {0}, only 17 is supported")]
    UnsupportedVersion(u32),
    #[error("Unexpected end of savefile while reading data")]
    EOF,
    #[error("found unexpected bytes at end")]
    UnexpectedAtEnd,
    #[error("could not decode utf-8")]
    UTF8(std::str::Utf8Error),
    #[error("failed to decompress: {0}")]
    LZ4(#[from] lz4_flex::block::DecompressError),

    #[error("unexpected lua state")]
    LuaError,
}

pub fn read_bytes_array<const N: usize>(data: &mut &[u8]) -> Result<[u8; N]> {
    if data.len() < N {
        return Err(Error::EOF);
    }

    let (bytes, rest) = data.split_at(N);
    *data = rest;
    Ok(bytes.try_into().unwrap())
}
pub fn read_bytes<'i>(data: &mut &'i [u8], n: usize) -> Result<&'i [u8]> {
    if data.len() < n {
        return Err(Error::EOF);
    }

    let (bytes, rest) = data.split_at(n);
    *data = rest;
    Ok(bytes)
}

pub fn read_u8(data: &mut &[u8]) -> Result<u8> {
    let (first, rest) = data.split_first().ok_or(Error::EOF)?;
    *data = rest;
    Ok(*first)
}

pub fn read_bool(data: &mut &[u8]) -> Result<bool> {
    let val = read_u8(data)?;
    Ok(val != 0)
}
pub fn read_u32(data: &mut &[u8]) -> Result<u32> {
    let bytes = read_bytes_array::<4>(data)?;
    Ok(u32::from_le_bytes(bytes))
}
pub fn read_u64(data: &mut &[u8]) -> Result<u64> {
    let bytes = read_bytes_array::<8>(data)?;
    Ok(u64::from_le_bytes(bytes))
}
pub fn read_f64(data: &mut &[u8]) -> Result<f64> {
    let bytes = read_bytes_array::<8>(data)?;
    Ok(f64::from_le_bytes(bytes))
}

pub fn read_str_prefix<'i>(data: &mut &'i [u8]) -> Result<&'i str> {
    let length = read_u32(data)?;
    let data = read_bytes(data, length as usize)?;
    let str = std::str::from_utf8(data).map_err(Error::UTF8)?;
    Ok(str)
}

pub fn read_array<'i, T>(
    data: &mut &'i [u8],
    f: impl Fn(&mut &'i [u8]) -> Result<T>,
) -> Result<Vec<T>> {
    let len = read_u32(data)?;

    let mut items = Vec::with_capacity(len as usize);
    for _ in 0..len {
        let item = f(data)?;
        items.push(item);
    }

    Ok(items)
}
