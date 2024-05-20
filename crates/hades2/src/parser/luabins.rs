use std::borrow::Cow;

use super::*;

#[derive(PartialEq, PartialOrd, Clone)]
pub enum Value<'a> {
    Nil,
    Bool(bool),
    Number(f64),
    String(Cow<'a, str>),
    Table(Vec<(Value<'a>, Value<'a>)>),
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
                (Value::Table(f0_self), Value::Table(f0_other)) => f0_self.cmp(&f0_other),
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

impl Value<'_> {
    pub fn as_table(&self) -> Option<&[(Value<'_>, Value<'_>)]> {
        match self {
            Value::Table(entries) => Some(entries.as_slice()),
            _ => None,
        }
    }
    pub fn visit(&self, include_keys: bool, f: &mut impl FnMut(&Value<'_>)) {
        f(self);
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

            pairs.sort_by(|(a, _), (b, _)| a.cmp(b));

            Value::Table(pairs)
        }
        _ => return Err(Error::LuaError),
    };

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
