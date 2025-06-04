use std::collections::BTreeMap;
use std::collections::HashMap;
use std::io::Cursor;

use anyhow::Context;
use serde::Deserialize;
use serde_json::json;
use serde_json::{Map, Value};
use tg_bytes_util::{ExistFlag, FromBytes};
use varint_rs::VarintReader;
mod custom_parser;

#[derive(Debug, Deserialize)]
pub enum DataDefine {
    Class {
        skip_existflag_check: Option<bool>,
        fields: Vec<DataField>,
        interfaces: Vec<String>,
    },
    Struct {
        fields: Vec<DataField>,
        interfaces: Vec<String>,
    },
    Typeindex {
        base: String,
        descendants: BTreeMap<u64, ValueKind>,
    },
    Enum(String, BTreeMap<String, String>),
}

#[derive(Debug, Deserialize)]
pub struct DataField {
    pub field_name: String,
    pub data_type: ValueKind,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub enum ValueKind {
    Primitive(String),
    Array(Box<ValueKind>),
    Dictionary(Box<ValueKind>, Box<ValueKind>),
    Class(String),
    Other(),
}

pub struct DynamicParser<'a> {
    pub types: &'a HashMap<String, DataDefine>,
    pub cursor: Cursor<&'a Vec<u8>>,
}

impl<'a> DynamicParser<'a> {
    pub fn new(types: &'a HashMap<String, DataDefine>, data: &'a Vec<u8>) -> Self {
        Self {
            types,
            cursor: Cursor::new(data),
        }
    }

    pub fn parse(&mut self, kind: &ValueKind, include_type: bool) -> anyhow::Result<Value> {
        if self.remaining() < 1 {
            tracing::debug!("{:?} buffer is empty", kind);

            return Ok(match kind {
                ValueKind::Primitive(_) => Value::Number(0.into()),
                ValueKind::Array(_) => Value::Array(Vec::with_capacity(0)),
                ValueKind::Dictionary(_, _) | ValueKind::Class(_) => {
                    Value::Object(Map::with_capacity(0))
                }
                ValueKind::Other() => Value::Null,
            });
        }

        Ok(match kind {
            ValueKind::Primitive(cs_type) => match cs_type.as_str() {
                "byte" => Value::Number(self.cursor.read_u8_varint()?.into()),
                "sbyte" => Value::Number(self.cursor.read_i8_varint()?.into()),
                "short" => Value::Number(self.cursor.read_i16_varint()?.into()),
                "ushort" => Value::Number(self.cursor.read_u16_varint()?.into()),
                "int" => Value::Number(self.cursor.read_i32_varint()?.into()),
                "uint" => Value::Number(self.cursor.read_u32_varint()?.into()),
                "long" => Value::Number(self.cursor.read_i64_varint()?.into()),
                "ulong" => Value::Number(serde_json::Number::from(self.cursor.read_u64_varint()?)),
                "float" => {
                    let raw = f32::from_bytes(&mut self.cursor)? as f64;
                    let sanitized = if raw.is_finite() { raw } else { 0.0 };
                    let number = serde_json::Number::from_f64(sanitized)
                        .ok_or_else(|| anyhow::anyhow!("float should always be finite"))?;
                    Value::Number(number)
                }
                "double" => Value::Number(
                    serde_json::Number::from_f64(f64::from_bytes(&mut self.cursor)?)
                        .ok_or_else(|| anyhow::anyhow!("invalid double"))?,
                ),
                "bool" => Value::Bool(bool::from_bytes(&mut self.cursor)?),
                "string" => Value::String(String::from_bytes(&mut self.cursor)?),
                other => return Err(anyhow::format_err!("unhandled primitive: {other}")),
            },
            ValueKind::Dictionary(key, value) => {
                tracing::debug!(
                    "ValueKind::Dictionary(cursor_pos: {}) -> Dictionary<{:?}, {:?}>",
                    self.cursor.position(),
                    key,
                    value
                );

                let length = self.cursor.read_i64_varint()? as usize;

                tracing::debug!(
                    "ValueKind::Dictionary(cursor_pos: {}) -> Dictionary length: {}",
                    self.cursor.position(),
                    length
                );

                if length > 1_000_000 {
                    return Err(anyhow::format_err!("attempting to allocate large memory!"));
                }

                let mut output = Map::with_capacity(length);

                for _ in 0..length {
                    let key = self.parse(key, false)?;
                    output.insert(
                        if let Value::String(s) = key {
                            s
                        } else {
                            key.to_string()
                        },
                        self.parse(value, false)?,
                    );
                }

                Value::Object(output)
            }
            ValueKind::Array(value) => {
                tracing::debug!(
                    "ValueKind::Array(cursor_pos: {}) -> {:?}[]",
                    self.cursor.position(),
                    value
                );

                let length = self.cursor.read_i64_varint()? as usize;

                tracing::debug!(
                    "ValueKind::Array(cursor_pos: {}) -> Array length: {}",
                    self.cursor.position(),
                    length
                );

                if length > 1_000_000 {
                    return Err(anyhow::format_err!("attempting to allocate large memory!"));
                }

                let mut output = Vec::with_capacity(length);

                for _ in 0..length {
                    output.push(self.parse(value, false)?);
                }

                Value::Array(output)
            }
            ValueKind::Class(class_name) => {
                tracing::debug!(
                    "ValueKind::Class(cursor_pos: {}) -> {}",
                    self.cursor.position(),
                    class_name
                );

                if let Some(custom) = custom_parser::CUSTOM_PARSER.get(class_name.as_str()) {
                    return custom(self);
                }

                let Some(define) = self.types.get(class_name) else {
                    return Err(anyhow::format_err!("unhandled type: {}", &class_name));
                };

                let mut result = self.parse_class_kind(define)?;

                if include_type {
                    result.as_object_mut().and_then(|f| {
                        f.shift_insert(
                            0,
                            "$type".into(),
                            Value::String(
                                class_name
                                    .strip_suffix("Inner")
                                    .unwrap_or(class_name)
                                    .to_string(),
                            ),
                        )
                    });
                }

                result
            }
            _ => return Err(anyhow::format_err!("unknown data kind!")),
        })
    }

    fn parse_class_kind(&mut self, data_type: &DataDefine) -> anyhow::Result<Value> {
        Ok(match data_type {
            DataDefine::Class {
                skip_existflag_check,
                fields,
                interfaces: _,
            } => {
                if skip_existflag_check.is_some() {
                    return Ok(json!({}));
                }

                let exist_flag = ExistFlag::new(&mut self.cursor, fields.len())?;
                let mut output = Map::with_capacity(fields.len());
                for (i, field) in fields.iter().enumerate() {
                    if exist_flag.exists(i) {
                        tracing::debug!(
                            "DataDefine::Class(cursor_pos: {}) -> Key: {}",
                            self.cursor.position(),
                            field.field_name
                        );

                        let value = self.parse(&field.data_type, false)?;

                        tracing::debug!(
                            "DataDefine::Class(cursor_pos: {}) -> Value: {:?}",
                            self.cursor.position(),
                            value
                        );

                        output.insert(field.field_name.to_string(), value);
                    } else {
                        tracing::debug!(
                            "DataDefine::Class(cursor_pos: {}) -> Field not exist! key: {}",
                            self.cursor.position(),
                            field.field_name
                        );
                    }
                }
                Value::Object(output)
            }
            DataDefine::Struct {
                fields,
                interfaces: _,
            } => {
                let mut output = Map::with_capacity(fields.len());
                for field in fields {
                    tracing::debug!(
                        "DataDefine::Struct(cursor_pos: {}) -> Key: {}",
                        self.cursor.position(),
                        field.field_name
                    );

                    let value = self.parse(&field.data_type, false)?;

                    tracing::debug!(
                        "DataDefine::Struct(cursor_pos: {}) -> Value: {:?}",
                        self.cursor.position(),
                        value
                    );

                    output.insert(field.field_name.to_string(), value);
                }
                Value::Object(output)
            }
            DataDefine::Typeindex { base, descendants } => {
                tracing::debug!(
                    "DataDefine::Typeindex(cursor_pos: {})",
                    self.cursor.position()
                );

                let typeindex = self
                    .cursor
                    .read_u64_varint()
                    .context("typeindex reading failed")?;

                let Some(descendant) = descendants.get(&typeindex) else {
                    return Err(anyhow::format_err!(
                        "typeindex not exist! dict: {:?} type index: {}",
                        descendants,
                        typeindex
                    ));
                };

                if let ValueKind::Class(descendant) = descendant
                    && let Some(DataDefine::Typeindex {
                        base: _,
                        descendants,
                    }) = self.types.get(descendant)
                    && let Some(descendant) = descendants.get(&0)
                {
                    return self.parse(descendant, true);
                }

                tracing::debug!(
                    "DataDefine::Typeindex(cursor_pos: {}) -> {} typeindex: {typeindex}",
                    self.cursor.position(),
                    base
                );

                return self.parse(descendant, true);
            }
            DataDefine::Enum(enum_type, enums) => {
                let enum_value = match enum_type.as_str() {
                    "ulong" => {
                        let discriminant = self.cursor.read_u64_varint()?;
                        if let Some(enum_value) = enums.get(&discriminant.to_string()) {
                            enum_value
                        } else {
                            tracing::debug!(
                                "enum_value not exist! enums: {:?} discriminant: {}",
                                enums,
                                discriminant
                            );
                            &discriminant.to_string()
                        }
                    }
                    "int" => {
                        let discriminant = self.cursor.read_i32_varint()?;
                        if let Some(discriminant) = enums.get(&discriminant.to_string()) {
                            discriminant
                        } else {
                            tracing::warn!(
                                "enum_value not exist! enums: {:?} discriminant: {}",
                                enums,
                                discriminant
                            );
                            &format!("{discriminant}")
                        }
                    }
                    "uint" => {
                        let discriminant = self.cursor.read_u32_varint()?;
                        if let Some(discriminant) = enums.get(&discriminant.to_string()) {
                            discriminant
                        } else {
                            tracing::warn!(
                                "enum_value not exist! enums: {:?} discriminant: {}",
                                enums,
                                discriminant
                            );
                            &format!("{discriminant}")
                        }
                    }
                    "ushort" => {
                        let discriminant = self.cursor.read_u16_varint()?;
                        if let Some(discriminant) = enums.get(&discriminant.to_string()) {
                            discriminant
                        } else {
                            tracing::debug!(
                                "enum_value not exist! enums: {:?} discriminant: {}",
                                enums,
                                discriminant
                            );
                            &format!("{discriminant}")
                        }
                    }
                    _ => return Err(anyhow::format_err!("unsupported enum type: {}", enum_type)),
                };
                Value::String(enum_value.into())
            }
        })
    }

    #[inline]
    fn remaining(&self) -> usize {
        self.cursor.get_ref().len() - self.cursor.position() as usize
    }
}
