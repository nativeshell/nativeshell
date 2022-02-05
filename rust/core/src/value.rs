use std::{
    cmp::Ordering,
    collections::HashMap,
    convert::{Infallible, TryFrom},
    fmt::Display,
    hash::Hash,
    num::TryFromIntError,
    ops::Deref,
};

#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    String(String),
    U8List(Vec<u8>),
    I32List(Vec<i32>),
    I64List(Vec<i64>),
    F32List(Vec<f32>),
    F64List(Vec<f64>),
    List(Vec<Value>),
    Map(ValueTupleList),
}

/// Wrapper for Value tuple that ensures that the underyling list is sorted
#[derive(Clone, Debug, PartialEq, PartialOrd, Hash)]
pub struct ValueTupleList(Vec<(Value, Value)>);

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
impl_from!(Value::I64, i8);
impl_from!(Value::I64, u8);
impl_from!(Value::I64, i16);
impl_from!(Value::I64, u16);
impl_from!(Value::I64, i32);
impl_from!(Value::I64, u32);
impl_from!(Value::I64, i64);
impl_from!(Value::F64, f32);
impl_from!(Value::F64, f64);
impl_from!(Value::String, String);
impl_from!(Value::String, &str);
impl_from!(Value::U8List, Vec<u8>);
impl_from!(Value::I32List, Vec<i32>);
impl_from!(Value::I64List, Vec<i64>);
impl_from!(Value::F32List, Vec<f32>);
impl_from!(Value::F64List, Vec<f64>);
impl_from!(Value::List, Vec<Value>);
impl_from!(Value::Map, Vec<(Value, Value)>);
impl_from!(Value::Map, HashMap<Value, Value>);

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(v) => v.into(),
            None => Value::Null,
        }
    }
}

impl From<()> for Value {
    fn from(_: ()) -> Self {
        Value::Null
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TryFromError {
    BadType,
    IntConversionError,
    OtherError(String),
}

impl Display for TryFromError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TryFromError::BadType => write!(f, "Could not convert value from unrelated type."),
            TryFromError::IntConversionError => {
                write!(f, "Could not convert integer value to a smaller type.")
            }
            TryFromError::OtherError(str) => {
                write!(f, "{}", str)
            }
        }
    }
}

impl std::error::Error for TryFromError {}

impl From<TryFromIntError> for TryFromError {
    fn from(_: TryFromIntError) -> Self {
        Self::IntConversionError
    }
}

impl From<Infallible> for TryFromError {
    fn from(_: Infallible) -> Self {
        panic!("Must never happen")
    }
}

macro_rules! impl_try_from {
    ($variant:path, $for_type:ty) => {
        impl TryFrom<Value> for $for_type {
            type Error = TryFromError;
            fn try_from(v: Value) -> Result<Self, Self::Error> {
                match v {
                    $variant(d) => Ok(d.into()),
                    _ => Err(TryFromError::BadType),
                }
            }
        }
    };
}

macro_rules! impl_try_from2 {
    ($variant:path, $for_type:ty) => {
        impl TryFrom<Value> for $for_type {
            type Error = TryFromError;
            fn try_from(v: Value) -> Result<Self, Self::Error> {
                use ::core::convert::TryInto;
                match v {
                    $variant(d) => Ok(d.try_into().map_err(TryFromError::from)?),
                    _ => Err(TryFromError::BadType),
                }
            }
        }
    };
}

impl TryFrom<Value> for () {
    type Error = TryFromError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Null => Ok(()),
            _ => Err(TryFromError::BadType),
        }
    }
}

impl_try_from!(Value::Bool, bool);
impl_try_from2!(Value::I64, u8);
impl_try_from2!(Value::I64, i8);
impl_try_from2!(Value::I64, u16);
impl_try_from2!(Value::I64, i16);
impl_try_from2!(Value::I64, i32);
impl_try_from2!(Value::I64, u32);
impl_try_from!(Value::I64, i64);
impl_try_from!(Value::F64, f64);
impl_try_from!(Value::String, String);
impl_try_from!(Value::U8List, Vec<u8>);
impl_try_from!(Value::I32List, Vec<i32>);
impl_try_from!(Value::I64List, Vec<i64>);
impl_try_from!(Value::F32List, Vec<f32>);
impl_try_from!(Value::F64List, Vec<f64>);
impl_try_from!(Value::List, Vec<Value>);
impl_try_from!(Value::Map, ValueTupleList);
impl_try_from!(Value::Map, Vec<(Value, Value)>);
impl_try_from!(Value::Map, HashMap<Value, Value>);

impl Eq for Value {}

fn hash_f64<H: std::hash::Hasher>(value: f64, state: &mut H) {
    // normalize NAN
    let value: f64 = if value.is_nan() { f64::NAN } else { value };
    let transmuted: u64 = value.to_bits();
    state.write_u64(transmuted);
}

fn hash_f32<H: std::hash::Hasher>(value: f32, state: &mut H) {
    // normalize NAN
    let value: f32 = if value.is_nan() { f32::NAN } else { value };
    let transmuted: u32 = value.to_bits();
    state.write_u32(transmuted);
}

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
            Value::F32List(v) => v.iter().for_each(|x| hash_f32(*x, state)),
            Value::F64List(v) => v.iter().for_each(|x| hash_f64(*x, state)),
            Value::List(v) => v.hash(state),
            Value::Map(v) => v.hash(state),
        }
    }
}

impl ValueTupleList {
    pub fn new(mut value: Vec<(Value, Value)>) -> Self {
        // Sort the list so tht hash and compares are deterministic
        if value
            .windows(2)
            .any(|w| w[0].0.partial_cmp(&w[1].0) != Some(Ordering::Less))
        {
            value.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        }
        Self(value)
    }
}

impl Deref for ValueTupleList {
    type Target = Vec<(Value, Value)>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for ValueTupleList {
    type Item = (Value, Value);

    type IntoIter = std::vec::IntoIter<(Value, Value)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<Vec<(Value, Value)>> for ValueTupleList {
    fn from(vec: Vec<(Value, Value)>) -> Self {
        Self::new(vec)
    }
}

impl From<HashMap<Value, Value>> for ValueTupleList {
    fn from(map: HashMap<Value, Value>) -> Self {
        let vec: Vec<_> = map.into_iter().collect();
        vec.into()
    }
}

impl From<ValueTupleList> for Vec<(Value, Value)> {
    fn from(list: ValueTupleList) -> Self {
        list.0
    }
}

impl From<ValueTupleList> for HashMap<Value, Value> {
    fn from(value: ValueTupleList) -> Self {
        value.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::Value;

    #[test]
    fn test_equality() {
        let v1 = Value::Map(vec![("key1".into(), 10.into()), ("key2".into(), 20.into())].into());
        let v2 = Value::Map(vec![("key2".into(), 20.into()), ("key1".into(), 10.into())].into());
        assert_eq!(v1, v2);
    }
}
