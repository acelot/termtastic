use std::{collections::hash_map, fmt::Display};

use serde::{Deserializer, de::*};

use crate::types::{FormData, FormValue};

pub fn from_formdata<'a, T: Deserialize<'a>>(
    data: &'a FormData,
) -> Result<T, FormDataDeserializerError> {
    let deserializer = FormDataDeserializer::new(data);
    T::deserialize(deserializer)
}

#[derive(thiserror::Error, Debug)]
pub enum FormDataDeserializerError {
    #[error("serde error: {0}")]
    Serde(String),
    #[error("invalid type: expected - {expected}, actual - {actual}")]
    InvalidType { expected: String, actual: String },
    #[error("unsupported type: {0}")]
    UnsupportedType(&'static str),
}

impl<'a> Error for FormDataDeserializerError {
    fn custom<T>(msg: T) -> Self
    where
        T: Display,
    {
        Self::Serde(msg.to_string())
    }
}

pub struct FormDataDeserializer<'a> {
    data: &'a FormData,
}

impl<'a> FormDataDeserializer<'a> {
    pub fn new(data: &'a FormData) -> Self {
        Self { data }
    }
}

impl<'a> Deserializer<'a> for FormDataDeserializer<'a> {
    type Error = FormDataDeserializerError;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("any"))
    }

    fn deserialize_bool<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("bool"))
    }

    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("i8"))
    }

    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("i16"))
    }

    fn deserialize_i32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("i32"))
    }

    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("i64"))
    }

    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("u8"))
    }

    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("u16"))
    }

    fn deserialize_u32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("u32"))
    }

    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("u64"))
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("f32"))
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("f64"))
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("char"))
    }

    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("str"))
    }

    fn deserialize_string<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("string"))
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("bytes"))
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("byte_buf"))
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("option"))
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("unit"))
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("unit_struct"))
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("newtype_struct"))
    }

    fn deserialize_seq<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("seq"))
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("tuple"))
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("tuple_struct"))
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        visitor.visit_map(FormDataAccessImpl {
            entries: self.data.into_iter(),
            current_key: None,
        })
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("enum"))
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("identifier"))
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("ignored_any"))
    }
}

struct FormDataAccessImpl<'a> {
    entries: hash_map::Iter<'a, &'static str, FormValue>,
    current_key: Option<&'static str>,
}

impl<'a> MapAccess<'a> for FormDataAccessImpl<'a> {
    type Error = FormDataDeserializerError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'a>,
    {
        if let Some((key, _value)) = self.entries.next() {
            self.current_key = Some(key);
            seed.deserialize(key.into_deserializer()).map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'a>,
    {
        let _ = self
            .current_key
            .take()
            .expect("next_value called before next_key");

        let value = self.entries.next().expect("value not found").1;
        let value_deserializer = FormValueDeserializer { value };

        seed.deserialize(value_deserializer)
    }
}

struct FormValueDeserializer<'a> {
    value: &'a FormValue,
}

impl<'a> Deserializer<'a> for FormValueDeserializer<'a> {
    type Error = FormDataDeserializerError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        match self.value {
            FormValue::String(v) => visitor.visit_string(v.to_owned()),
            FormValue::Int32(v) => visitor.visit_i32(*v),
            FormValue::UnsignedInt32(v) => visitor.visit_u32(*v),
            FormValue::Float32(v) => visitor.visit_f32(*v),
            FormValue::Bool(v) => visitor.visit_bool(*v),
            FormValue::VecOfFormValue(vec) => {
                visitor.visit_seq(FormValueSeqAccess { vec, index: 0 })
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        match self.value {
            FormValue::Bool(v) => visitor.visit_bool(*v),
            other => Err(FormDataDeserializerError::InvalidType {
                expected: "bool".to_owned(),
                actual: format!("{:?}", other),
            }),
        }
    }

    fn deserialize_i8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("i8"))
    }

    fn deserialize_i16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("i16"))
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        match self.value {
            FormValue::Int32(v) => visitor.visit_i32(*v),
            other => Err(FormDataDeserializerError::InvalidType {
                expected: "i32".to_owned(),
                actual: format!("{:?}", other),
            }),
        }
    }

    fn deserialize_i64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("i64"))
    }

    fn deserialize_u8<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("u8"))
    }

    fn deserialize_u16<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("u16"))
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        match self.value {
            FormValue::UnsignedInt32(v) => visitor.visit_u32(*v),
            other => Err(FormDataDeserializerError::InvalidType {
                expected: "u32".to_owned(),
                actual: format!("{:?}", other),
            }),
        }
    }

    fn deserialize_u64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("u64"))
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        match self.value {
            FormValue::Float32(v) => visitor.visit_f32(*v),
            other => Err(FormDataDeserializerError::InvalidType {
                expected: "f32".to_owned(),
                actual: format!("{:?}", other),
            }),
        }
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("f64"))
    }

    fn deserialize_char<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("char"))
    }

    fn deserialize_str<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("str"))
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        match self.value {
            FormValue::String(v) => visitor.visit_string(v.to_owned()),
            other => Err(FormDataDeserializerError::InvalidType {
                expected: "f32".to_owned(),
                actual: format!("{:?}", other),
            }),
        }
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("bytes"))
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("byte_buf"))
    }

    fn deserialize_option<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("option"))
    }

    fn deserialize_unit<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("unit"))
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("unit_struct"))
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("newtype_struct"))
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        match self.value {
            FormValue::VecOfFormValue(v) => {
                visitor.visit_seq(FormValueSeqAccess { vec: v, index: 0 })
            }
            other => Err(FormDataDeserializerError::InvalidType {
                expected: "seq".to_owned(),
                actual: format!("{:?}", other),
            }),
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("tuple"))
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("tuple_struct"))
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("map"))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("struct"))
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("enum"))
    }

    fn deserialize_identifier<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("identifier"))
    }

    fn deserialize_ignored_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'a>,
    {
        Err(FormDataDeserializerError::UnsupportedType("ignored_any"))
    }
}

struct FormValueSeqAccess<'a> {
    vec: &'a [FormValue],
    index: usize,
}

impl<'a> SeqAccess<'a> for FormValueSeqAccess<'a> {
    type Error = FormDataDeserializerError;

    fn next_element_seed<S>(&mut self, seed: S) -> Result<Option<S::Value>, Self::Error>
    where
        S: DeserializeSeed<'a>,
    {
        if self.index < self.vec.len() {
            let deserializer = FormValueDeserializer {
                value: &self.vec[self.index],
            };
            self.index += 1;
            seed.deserialize(deserializer).map(Some)
        } else {
            Ok(None)
        }
    }
}
