mod deserializer;
mod serializer;

use std::{convert::TryFrom, fmt};

use std::{collections::HashMap, hash::Hash};

pub use self::{
    deserializer::{from_value, from_value_owned},
    serializer::to_value,
};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    U8List(Vec<u8>),
    I32List(Vec<i32>),
    I64List(Vec<i64>),
    F64List(Vec<f64>),
    List(Vec<Value>),
    Map(HashMap<Value, Value>),
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

macro_rules! impl_from {
    ($variant:path, $for_type:ty) => {
        impl From<$for_type> for Value {
            fn from(v: $for_type) -> Value {
                $variant(v.into())
            }
        }
    };
}

impl_from!(Value::Bool, bool);
impl_from!(Value::I64, i64);
impl_from!(Value::I64, u32);
impl_from!(Value::F64, f32);
impl_from!(Value::F64, f64);
impl_from!(Value::String, String);
impl_from!(Value::String, &str);
impl_from!(Value::U8List, Vec<u8>);
impl_from!(Value::I32List, Vec<i32>);
impl_from!(Value::I64List, Vec<i64>);
impl_from!(Value::F64List, Vec<f64>);
impl_from!(Value::List, Vec<Value>);
impl_from!(Value::Map, HashMap<Value, Value>);

impl Eq for Value {}

fn hash_f64<H: std::hash::Hasher>(value: f64, state: &mut H) {
    // normalize NAN
    let value: f64 = if value.is_nan() { f64::NAN } else { value };
    let transmuted: u64 = value.to_bits();
    state.write_u64(transmuted);
}

fn hash_map<H: std::hash::Hasher>(map: &HashMap<Value, Value>, state: &mut H) {
    for (key, value) in map {
        key.hash(state);
        value.hash(state);
    }
}

#[allow(renamed_and_removed_lints)]
#[allow(clippy::derive_hash_xor_eq)]
impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Value::Null => state.write_u64(640),
            Value::Bool(v) => v.hash(state),
            Value::I64(v) => v.hash(state),
            Value::F64(v) => hash_f64(*v, state),
            Value::String(v) => v.hash(state),
            Value::U8List(v) => v.hash(state),
            Value::I32List(v) => v.hash(state),
            Value::I64List(v) => v.hash(state),
            Value::F64List(v) => v.iter().for_each(|x| hash_f64(*x, state)),
            Value::List(v) => v.hash(state),
            Value::Map(v) => hash_map(v, state),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValueError {
    Message(String),
    ConversionError,
    WrongType,
    NoList,
    NoMap,
}

impl fmt::Display for ValueError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ValueError::Message(s) => write!(f, "{s}"),
            ValueError::ConversionError => write!(f, "Value can't be converted to target type"),
            ValueError::WrongType => write!(f, "Value is of wrong type"),
            ValueError::NoList => write!(f, "Value is not a list"),
            ValueError::NoMap => write!(f, "Value is not a map"),
        }
    }
}

impl serde::ser::Error for ValueError {
    fn custom<T>(msg: T) -> Self
    where
        T: fmt::Display,
    {
        ValueError::Message(msg.to_string())
    }
}

impl serde::de::Error for ValueError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        ValueError::Message(msg.to_string())
    }
}

impl std::error::Error for ValueError {}

impl serde::Serialize for Value {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::I64(i) => serializer.serialize_i64(*i),
            Value::F64(f) => serializer.serialize_f64(*f),
            Value::String(s) => serializer.serialize_str(s.as_str()),
            Value::U8List(vec) => serializer.serialize_bytes(vec),
            Value::I32List(vec) => vec.serialize(serializer),
            Value::I64List(vec) => vec.serialize(serializer),
            Value::F64List(vec) => vec.serialize(serializer),
            Value::List(vec) => vec.serialize(serializer),
            Value::Map(map) => {
                use serde::ser::SerializeMap;
                let mut m = serializer.serialize_map(Some(map.len()))?;
                for (k, v) in map {
                    m.serialize_entry(k, v)?;
                }
                m.end()
            }
        }
    }
}

struct ValueVisitor;

impl<'de> serde::de::Visitor<'de> for ValueVisitor {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("any valid JSON value")
    }

    #[inline]
    fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    #[inline]
    fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
        Ok(Value::I64(value))
    }

    #[inline]
    fn visit_u64<E>(self, value: u64) -> Result<Value, E>
    where
        E: serde::de::Error,
    {
        if let Ok(i) = i64::try_from(value) {
            Ok(Value::I64(i))
        } else {
            Err(E::custom("Number too large for i64"))
        }
    }

    #[inline]
    fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
        Ok(Value::F64(value))
    }

    #[inline]
    fn visit_str<E>(self, value: &str) -> Result<Value, E> {
        Ok(Value::String(value.into()))
    }

    #[inline]
    fn visit_string<E>(self, value: String) -> Result<Value, E> {
        Ok(Value::String(value))
    }

    #[inline]
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Value, E> {
        Ok(Value::U8List(v.into()))
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    #[inline]
    fn visit_some<D>(self, deserializer: D) -> Result<Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        serde::Deserialize::deserialize(deserializer)
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    #[inline]
    fn visit_seq<V>(self, mut visitor: V) -> Result<Value, V::Error>
    where
        V: serde::de::SeqAccess<'de>,
    {
        let mut vec = Vec::new();
        while let Some(elem) = visitor.next_element()? {
            vec.push(elem);
        }
        Ok(Value::List(vec))
    }

    #[inline]
    fn visit_map<V>(self, mut visitor: V) -> Result<Value, V::Error>
    where
        V: serde::de::MapAccess<'de>,
    {
        let mut map = HashMap::new();
        while let Some((k, v)) = visitor.next_entry()? {
            map.insert(k, v);
        }
        Ok(Value::Map(map))
    }
}

impl<'de> serde::Deserialize<'de> for Value {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor)
    }
}
