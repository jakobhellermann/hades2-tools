use std::collections::HashSet;

use super::*;

pub enum Value<'a> {
    Nil,
    Bool(bool),
    Number(f64),
    String(&'a str),
    Table(Vec<(Value<'a>, Value<'a>)>),
}

impl<'a> std::fmt::Debug for Value<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Nil => write!(f, "Nil"),
            Self::Bool(val) => write!(f, "{}", val),
            Self::Number(val) => write!(f, "{}", val),
            Self::String(val) => f.write_str(val),
            // Self::Table(val) => f.debug_tuple("Table").field(val).finish(),
            Self::Table(table) => {
                let mut map = f.debug_map();
                for (key, val) in table {
                    map.entry(key, val);
                }
                map.finish()
            }
        }
    }
}

pub fn read_luabins<'i>(data: &mut &'i [u8]) -> Result<Vec<Value<'i>>> {
    let len = read_u8(data)?;

    if len > 250 {
        return Err(Error::LuaError);
    }

    let mut values = Vec::with_capacity(len as usize);
    for i in 0..len {
        let val = read_value(data)?;
        values.push(val);
    }

    if data.len() > 0 {
        return Err(Error::UnexpectedAtEnd);
    }

    Ok(values)
}

pub fn read_value<'i>(data: &mut &'i [u8]) -> Result<Value<'i>> {
    let ty = read_u8(data)?;
    let val = match ty {
        b'-' => Value::Nil,
        b'0' => Value::Bool(false),
        b'1' => Value::Bool(true),
        b'N' => {
            let number = read_f64(data)?;
            Value::Number(number)
        }
        b'S' => {
            let str = read_str_prefix(data)?;
            Value::String(str)
        }
        b'T' => {
            let array_size = read_u32(data)?;
            let hash_size = read_u32(data)?;
            let total_size = array_size + hash_size;

            let mut pairs = Vec::with_capacity(total_size as usize);

            for i in 0..total_size {
                let key = read_value(data)?;
                let val = read_value(data)?;

                pairs.push((key, val));
            }

            Value::Table(pairs)
        }
        _ => return Err(Error::LuaError),
    };

    Ok(val)
}
