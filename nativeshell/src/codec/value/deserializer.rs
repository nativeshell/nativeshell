use core::panic;
use std::collections::hash_map::Keys;

use super::{Value, ValueError};

use serde::de::IntoDeserializer;

type Result<T> = std::result::Result<T, ValueError>;

pub struct Deserializer<'de> {
    value: &'de Value,
}

impl<'de> Deserializer<'de> {
    pub fn new(value: &'de Value) -> Self {
        Self { value }
    }
}

impl<'a, 'de> serde::de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = ValueError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            Value::Null => visitor.visit_unit(),
            Value::Bool(b) => visitor.visit_bool(*b),
            Value::I64(i) => visitor.visit_i64(*i),
            Value::F64(f) => visitor.visit_f64(*f),
            Value::String(s) => visitor.visit_str(s.as_str()),
            Value::U8List(s) => visitor.visit_bytes(s),
            Value::I32List(_) => visitor.visit_seq(SeqAccess::new(self)),
            Value::I64List(_) => visitor.visit_seq(SeqAccess::new(self)),
            Value::F64List(_) => visitor.visit_seq(SeqAccess::new(self)),
            Value::List(_) => visitor.visit_seq(SeqAccess::new(self)),
            Value::Map(_) => visitor.visit_map(MapAccess::new(self)),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            Value::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            Value::String(s) => visitor.visit_enum(s.clone().into_deserializer()),
            Value::Map(m) => {
                if m.len() != 1 {
                    Err(ValueError::WrongType)
                } else {
                    visitor.visit_enum(EnumAccess::new(self))
                }
            }
            _ => Err(ValueError::WrongType),
        }
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    serde::forward_to_deserialize_any! {
        bool
        i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string bytes byte_buf unit unit_struct
        seq tuple tuple_struct map struct identifier ignored_any
    }
}

struct SeqAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    index: usize,
}

impl<'a, 'de> SeqAccess<'a, 'de> {
    pub fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de, index: 0 }
    }
}

impl<'a, 'de> serde::de::SeqAccess<'de> for SeqAccess<'a, 'de> {
    type Error = ValueError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.de.value {
            Value::I32List(vec) => {
                if vec.len() <= self.index {
                    return Ok(None);
                }
                self.index += 1;
                Ok(Some(
                    seed.deserialize(vec[self.index - 1].into_deserializer())?,
                ))
            }
            Value::I64List(vec) => {
                if vec.len() <= self.index {
                    return Ok(None);
                }
                self.index += 1;
                Ok(Some(
                    seed.deserialize(vec[self.index - 1].into_deserializer())?,
                ))
            }
            Value::F64List(vec) => {
                if vec.len() <= self.index {
                    return Ok(None);
                }
                self.index += 1;
                Ok(Some(
                    seed.deserialize(vec[self.index - 1].into_deserializer())?,
                ))
            }
            Value::List(vec) => {
                if vec.len() <= self.index {
                    return Ok(None);
                }
                self.index += 1;
                Ok(Some(seed.deserialize(&mut Deserializer::new(
                    &vec[self.index - 1],
                ))?))
            }
            _ => Err(ValueError::NoList),
        }
    }
}

struct MapAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    key_iter: Keys<'de, Value, Value>,
    next_value: Option<&'de Value>,
}

impl<'a, 'de> MapAccess<'a, 'de> {
    pub fn new(de: &'a mut Deserializer<'de>) -> Self {
        let map = if let Value::Map(map) = de.value {
            map
        } else {
            panic!("deserializer must have a map");
        };
        Self {
            de,
            key_iter: map.keys(),
            next_value: None,
        }
    }
}

impl<'a, 'de> serde::de::MapAccess<'de> for MapAccess<'a, 'de> {
    type Error = ValueError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        match self.de.value {
            Value::Map(map) => {
                let next = if let Some(k) = self.key_iter.next() {
                    k
                } else {
                    return Ok(None);
                };
                self.next_value.replace(map.get(next).unwrap());
                let deserializer = &mut Deserializer::new(next);
                Ok(Some(seed.deserialize(deserializer)?))
            }
            _ => Err(ValueError::NoMap),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut Deserializer::new(self.next_value.take().unwrap()))
    }
}

struct EnumAccess<'de> {
    name: &'de String,
    value_deserializer: Deserializer<'de>,
}

impl<'a, 'de> EnumAccess<'de> {
    pub fn new(de: &'a mut Deserializer<'de>) -> Self {
        let map = if let Value::Map(map) = de.value {
            map
        } else {
            panic!("deserializer must have a map");
        };
        if map.len() != 1 {
            panic!("map must have length 1");
        }
        let (name, sub_value) = map.iter().next().unwrap();
        match name {
            Value::String(name) => Self {
                name,
                value_deserializer: Deserializer::new(sub_value),
            },
            _ => {
                panic!("Name must be string")
            }
        }
    }
}

impl<'de> serde::de::EnumAccess<'de> for EnumAccess<'de> {
    type Error = ValueError;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let val = seed.deserialize(self.name.clone().into_deserializer())?;
        Ok((val, self))
    }
}

impl<'de> serde::de::VariantAccess<'de> for EnumAccess<'de> {
    type Error = ValueError;

    fn unit_variant(mut self) -> Result<()> {
        serde::de::Deserialize::deserialize(&mut self.value_deserializer)
    }

    fn newtype_variant_seed<V>(mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut self.value_deserializer)
    }

    fn tuple_variant<V>(mut self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::de::Deserializer::deserialize_seq(&mut self.value_deserializer, visitor)
    }

    fn struct_variant<V>(mut self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        serde::de::Deserializer::deserialize_struct(
            &mut self.value_deserializer,
            "",
            fields,
            visitor,
        )
    }
}

pub fn from_value<'a, T>(value: &'a Value) -> Result<T>
where
    T: serde::de::Deserialize<'a>,
{
    T::deserialize(&mut Deserializer::new(value))
}

pub fn from_value_owned<T>(value: &Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(&mut Deserializer::new(value))
}
