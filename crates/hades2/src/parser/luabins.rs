use std::borrow::Cow;

use super::*;

#[derive(PartialEq, PartialOrd, Clone)]
pub struct LuaTable<'a>(pub Vec<(Value<'a>, Value<'a>)>);

impl<'a> IntoIterator for &'a LuaTable<'a> {
    type Item = &'a (Value<'a>, Value<'a>);

    type IntoIter = core::slice::Iter<'a, (Value<'a>, Value<'a>)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<'a> LuaTable<'a> {
    pub fn iter(&self) -> impl Iterator<Item = &(Value<'a>, Value<'a>)> {
        self.0.iter()
    }
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut (Value<'a>, Value<'a>)> {
        self.0.iter_mut()
    }

    pub fn get_or_insert(&mut self, key: &str, insert: Value<'a>) -> &mut Value<'a> {
        let pos = self
            .0
            .iter()
            .position(|(k, _)| k.is_str(key))
            .unwrap_or_else(|| {
                let i = self.0.len();
                self.0.push((Value::String(key.to_owned().into()), insert));
                i
            });
        self.sort();
        &mut self.0[pos].1
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn sort(&mut self) {
        self.0.sort_by(|(a, a_val), (b, b_val)| {
            let primitive_first = b_val.is_primitive().cmp(&a_val.is_primitive());
            primitive_first.then_with(|| a.cmp(b))
        });
    }
}

#[derive(PartialEq, PartialOrd, Clone)]
pub enum Value<'a> {
    Nil,
    Bool(bool),
    Number(f64),
    String(Cow<'a, str>),
    Table(LuaTable<'a>),
}
impl std::cmp::Eq for Value<'_> {}
impl std::cmp::Ord for Value<'_> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let discriminant = |x: &Value| match *x {
            Value::Nil => 0,
            Value::Bool(_) => 1,
            Value::Number(_) => 2,
            Value::String(_) => 3,
            Value::Table(_) => 4,
        };

        match discriminant(self).cmp(&discriminant(other)) {
            std::cmp::Ordering::Equal => match (self, other) {
                (Value::Nil, Value::Nil) => std::cmp::Ordering::Equal,
                (Value::Bool(f0_self), Value::Bool(f0_other)) => f0_self.cmp(&f0_other),
                (Value::Number(f0_self), Value::Number(f0_other)) => f0_self.total_cmp(&f0_other),
                (Value::String(f0_self), Value::String(f0_other)) => {
                    let self_underscore = f0_self.starts_with('_');
                    let other_underscore = f0_other.starts_with('_');

                    other_underscore
                        .cmp(&self_underscore)
                        .then_with(|| f0_self.cmp(&f0_other))
                }
                (Value::Table(f0_self), Value::Table(f0_other)) => f0_self.0.cmp(&f0_other.0),
                _ => std::cmp::Ordering::Equal,
            },
            other => other,
        }
    }
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

impl<'l> Value<'l> {
    pub const EMPTY_TABLE: Self = Value::Table(LuaTable(Vec::new()));

    pub fn as_number(&self) -> Option<f64> {
        match *self {
            Value::Number(val) => Some(val),
            _ => None,
        }
    }
    pub fn as_number_mut(&mut self) -> Option<&mut f64> {
        match self {
            Value::Number(val) => Some(val),
            _ => None,
        }
    }

    pub fn as_table(&self) -> Option<&LuaTable<'l>> {
        match self {
            Value::Table(entries) => Some(&entries),
            _ => None,
        }
    }
    pub fn as_table_mut(&mut self) -> Option<&mut LuaTable<'l>> {
        match self {
            Value::Table(entries) => Some(entries),
            _ => None,
        }
    }

    pub fn visit(&self, include_keys: bool, f: &mut impl FnMut(&Value<'_>)) {
        match self {
            Value::Nil => {}
            Value::Bool(_) => {}
            Value::Number(_) => {}
            Value::String(_) => {}
            Value::Table(table) => {
                for (key, val) in table {
                    if include_keys {
                        key.visit(include_keys, f);
                    }
                    val.visit(include_keys, f);
                }
            }
        }
        f(self);
    }

    pub fn is_primitive(&self) -> bool {
        !matches!(self, Value::Table(_))
    }

    pub fn is_str(&self, val: &str) -> bool {
        match self {
            Value::String(str) => str.as_ref() == val,
            _ => false,
        }
    }

    pub fn primitive_to_str(&self) -> Option<Cow<'_, str>> {
        Some(match self {
            Value::Nil => "Nil".into(),
            Value::Bool(val) => val.to_string().into(),
            Value::Number(val) => val.to_string().into(),
            Value::String(val) => Cow::Borrowed(val.as_ref()),
            Value::Table(_) => return None,
        })
    }

    pub fn count(&self, include_keys: bool, f: &mut impl FnMut(&Value<'_>) -> bool) -> usize {
        let mut i = 0;
        self.visit(include_keys, &mut |value| {
            if f(value) {
                i += 1;
            }
        });
        i
    }
}

pub fn read_luabins<'i>(data: &mut &'i [u8]) -> Result<Vec<Value<'static>>> {
    let len = read_u8(data)?;

    if len > 250 {
        return Err(Error::LuaError);
    }

    let mut values = Vec::with_capacity(len as usize);
    for _ in 0..len {
        let val = read_value(data)?;
        values.push(val);
    }

    if data.len() > 0 {
        return Err(Error::UnexpectedAtEnd);
    }

    Ok(values)
}

pub fn read_value<'i>(data: &mut &'i [u8]) -> Result<Value<'static>> {
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
            // Value::String(Cow::Borrowed(str))
            Value::String(Cow::Owned(str.to_owned()))
        }
        b'T' => {
            let array_size = read_u32(data)?;
            let hash_size = read_u32(data)?;
            let total_size = array_size + hash_size;

            let mut pairs = Vec::with_capacity(total_size as usize);

            for _ in 0..total_size {
                let key = read_value(data)?;
                let val = read_value(data)?;

                pairs.push((key, val));
            }

            let mut table = LuaTable(pairs);
            table.sort();
            Value::Table(table)
        }
        _ => return Err(Error::LuaError),
    };

    /*let mut result = Vec::new();
    write::save_value(&mut result, &val);
    let same = result.as_slice() == &datacopy[..result.len()];
    if !same {
        let count = val.count(true, &mut |_| true);
        dbg!(count);
        if [3, 9, 13, 15].contains(&count) {
            dbg!(&val);
            std::fs::write("a", result.as_slice()).unwrap();
            std::fs::write("b", &datacopy[..result.len()]).unwrap();
            panic!();
        }
    }*/

    Ok(val)
}

#[cfg(feature = "serde")]
impl serde::Serialize for Value<'_> {
    fn serialize<S>(&self, serializer: S) -> std::prelude::v1::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match *self {
            Value::Nil => serializer.serialize_none(),
            Value::Bool(val) => serializer.serialize_bool(val),
            Value::Number(val) => serializer.serialize_f64(val),
            Value::String(ref val) => serializer.serialize_str(val),
            Value::Table(ref table) => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(table.len()))?;
                for (key, val) in table {
                    map.serialize_entry(key, val)?;
                }

                map.end()
            }
        }
    }
}

pub use write::write_luabins;

// from https://github.com/TannerRogalsky/luabins/blob/306510abeaec25784b606039202de4d88c72f48b/src/lib.rs#L172C1-L258C2 (MIT)
// since I already have my own parser without nom
pub mod write {
    use super::{LuaTable, Value};

    fn save_table(result: &mut Vec<u8>, table: &LuaTable) {
        // The canonical implementation of this function is here
        // https://github.com/lua/lua/blob/ad3942adba574c9d008c99ce2785a5af19d146bf/ltable.c#L889-L966
        fn array_size(table: &LuaTable) -> usize {
            let mut size = 0;

            for index in 1..=table.len() {
                let v = table.into_iter().find(|(key, _value)| match *key {
                    Value::Number(num) => index == num as usize,
                    _ => false,
                });
                if v.is_some() {
                    size = index;
                } else {
                    break;
                }
            }
            size
        }

        const USE_HADES_VERSION: bool = false;

        let (array_size, hash_size) = if USE_HADES_VERSION {
            let mut n_number = 0;

            let mut last_index = 0.0;
            let mut started_hashes = false;
            let mut is_consecutive = true;

            for (key, _) in table {
                match *key {
                    Value::Number(i) => {
                        if last_index == 0.0 && i != 1.0 {
                            is_consecutive = false;
                        }

                        is_consecutive &= !started_hashes;
                        is_consecutive &= i > last_index;
                        last_index = i;

                        n_number += 1;
                    }
                    _ if !started_hashes => started_hashes = true,
                    _ => {}
                }
            }

            if is_consecutive {
                (n_number, table.len() - n_number)
            } else {
                (0, table.len())
            }
        } else {
            let array = array_size(table);
            let hash_size = table.len() - array;

            (array, hash_size)
        };

        result.push(b'T');
        result.extend_from_slice(&((array_size as u32).to_le_bytes()));
        result.extend_from_slice(&((hash_size as u32).to_le_bytes()));

        // TODO: validate nesting depth
        for (key, value) in table {
            save_value(result, key);
            save_value(result, value);
        }
    }

    fn save_value(result: &mut Vec<u8>, value: &Value) {
        match value {
            Value::Nil => result.push(b'-'),
            Value::Bool(inner) => match *inner {
                false => result.push(b'0'),
                true => result.push(b'1'),
            },
            Value::Number(inner) => {
                result.push(b'N');
                result.extend_from_slice(&inner.to_le_bytes());
            }
            Value::String(inner) => {
                result.push(b'S');
                result.extend_from_slice(&(inner.len() as u32).to_le_bytes());
                result.extend_from_slice(inner.as_bytes());
            }
            Value::Table(table) => save_table(result, table),
        }
    }

    pub fn write_luabins<'a>(
        result: &mut Vec<u8>,
        data: impl Iterator<Item = &'a Value<'a>> + ExactSizeIterator,
    ) {
        result.push(data.len() as u8);
        for datum in data {
            save_value(result, datum);
        }
    }
}
