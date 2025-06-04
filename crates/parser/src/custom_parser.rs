use std::{
    collections::HashMap,
    io::{Read, Seek},
    sync::LazyLock,
};

use crate::{DynamicParser, ValueKind};
use base64::Engine;
use serde_json::{Map, Number, Value, json};
use varint_rs::VarintReader;

use tg_bytes_util::FromBytes;

type CustomParser =
    HashMap<&'static str, for<'a> fn(&mut DynamicParser<'a>) -> anyhow::Result<Value>>;

pub static CUSTOM_PARSER: LazyLock<CustomParser> = LazyLock::new(|| {
    let mut m: CustomParser = HashMap::with_capacity(7);
    m.insert("RPG.GameCore.FixPoint", fix_point_parser);
    m.insert("RPG.GameCore.DynamicValue", dynamic_value_parser);
    m.insert("LAHCFFKCOBC", dynamic_values_parser);
    m.insert("RPG.GameCore.DynamicFloat", dynamic_float_parser);
    m.insert("RPG.GameCore.ReadInfo", read_info_parser);
    m.insert("RPG.GameCore.JsonEnum", json_enum_parser);
    m.insert("RPG.Client.TextID", textid_parser);
    m
});

fn read_bytes<R: Read + Seek>(cursor: &mut R, len: usize) -> anyhow::Result<Vec<u8>> {
    let mut buffer = vec![0u8; len];
    cursor.read_exact(&mut buffer)?;
    Ok(buffer)
}

fn read_byte<R: Read + Seek>(cursor: &mut R) -> anyhow::Result<u8> {
    Ok(read_bytes(cursor, 1)?[0])
}

fn read_bool<R: Read + Seek>(cursor: &mut R) -> anyhow::Result<bool> {
    Ok(read_byte(cursor)? != 0)
}

fn fix_point_parser<'a>(parser: &mut DynamicParser<'a>) -> anyhow::Result<Value> {
    let value = parser.cursor.read_i64_varint()? as f32;
    Ok(json!({
        "Value": (value / (2f32).powf(32f32)) as f64
    }))
}

fn dynamic_value_parser<'a>(parser: &mut DynamicParser<'a>) -> anyhow::Result<Value> {
    let value_type = parser.cursor.read_i8_varint()?;

    let (r#type, value) = match value_type {
        0 => (
            String::from("Int32"),
            Value::Number(i32::from_bytes(&mut parser.cursor)?.into()),
        ),
        1 => (
            String::from("Float"),
            Value::Number(Number::from_f64(f32::from_bytes(&mut parser.cursor)? as f64).unwrap()),
        ),
        2 => (
            String::from("Boolean"),
            Value::Bool(bool::from_bytes(&mut parser.cursor)?),
        ),
        3 => {
            let length = parser.cursor.read_i64_varint()? as usize;
            if length > 1_000_000 {
                return Err(anyhow::format_err!("attempting to allocate large memory!"));
            }
            let mut result = Vec::with_capacity(length);
            for _ in 0..length {
                result.push(parser.parse(
                    &ValueKind::Class(String::from("RPG.GameCore.DynamicValue")),
                    false,
                )?);
            }
            (String::from("Array"), serde_json::to_value(result)?)
        }
        4 => {
            let length = parser.cursor.read_i64_varint()? as usize;
            if length > 1_000_000 {
                return Err(anyhow::format_err!("attempting to allocate large memory!"));
            }
            let mut result = Vec::with_capacity(length);
            for _ in 0..length {
                let _ = parser.cursor.read_i64_varint()?;
                let _ = parser.cursor.read_i64_varint()?;

                result.push(parser.parse(
                    &ValueKind::Class(String::from("RPG.GameCore.DynamicValue")),
                    false,
                )?);
            }
            (String::from("Map"), serde_json::to_value(result)?)
        }
        5 => (
            String::from("String"),
            Value::String(String::from_bytes(&mut parser.cursor)?),
        ),
        _ => (String::from("Null"), Value::Null),
    };

    Ok(json!({
       "Type": r#type,
       "Value": value
    }))
}

fn dynamic_values_parser<'a>(parser: &mut DynamicParser<'a>) -> anyhow::Result<Value> {
    let length = parser.cursor.read_u64_varint()? as usize;

    if length > 1_000_000 {
        return Err(anyhow::format_err!("attempting to allocate large memory!"));
    }

    let mut floats = Map::with_capacity(length);

    for _ in 0..length {
        let key = parser.parse(
            &ValueKind::Class(String::from("RPG.GameCore.StringHash")),
            false,
        )?;

        let v12 = bool::from_bytes(&mut parser.cursor)?;
        let value = if v12 {
            let v7 = dynamic_float_parser(parser)?;
            let v8 = dynamic_float_parser(parser)?;
            let v9 = dynamic_float_parser(parser)?;

            let read_info = read_info_parser(parser)?;
            json!({
                "v7": v7,
                "v8": v8,
                "v9": v9,
                "ReadInfo": read_info,
            })
        } else {
            let v24 = fix_point_parser(parser)?;

            let v17 = bool::from_bytes(&mut parser.cursor)?;
            let unk = if v17 {
                let v15 = fix_point_parser(parser)?;
                let v16 = fix_point_parser(parser)?;
                json!({
                    "v15": v15,
                    "v16": v16
                })
            } else {
                json!({})
            };

            let read_info = read_info_parser(parser)?;

            json!({
                "ReadInfo": read_info,
                "unk": unk,
                "v24": v24
            })
        };

        floats.insert(key.to_string(), value);
    }

    Ok(json!({
        "Floats": floats
    }))
}

fn dynamic_float_parser<'a>(parser: &mut DynamicParser<'a>) -> anyhow::Result<Value> {
    let is_dynamic = read_bool(&mut parser.cursor)?;

    Ok(if is_dynamic {
        let opcode_len = read_byte(&mut parser.cursor)? as usize;
        let opcodes = base64::engine::general_purpose::STANDARD
            .encode(read_bytes(&mut parser.cursor, opcode_len)?);

        let fixed_values = (0..read_byte(&mut parser.cursor)?)
            .map(|_| fix_point_parser(parser))
            .collect::<Result<Vec<_>, _>>()?;

        let dynamic_hashes = (0..read_byte(&mut parser.cursor)?)
            .map(|_| parser.cursor.read_i32_varint())
            .collect::<Result<Vec<_>, _>>()?;

        json!({
            "IsDynamic": true,
            "PostfixExpr": {
                "OpCodes": opcodes,
                "FixedValues": fixed_values,
                "DynamicHashes": dynamic_hashes
            }
        })
    } else {
        let fixed_value = fix_point_parser(parser)?;

        json!({
            "IsDynamic": false,
            "FixedValue": fixed_value
        })
    })
}

fn read_info_parser<'a>(parser: &mut DynamicParser<'a>) -> anyhow::Result<Value> {
    let has_read_info = read_bool(&mut parser.cursor)?;

    if has_read_info {
        let string = String::from_bytes(&mut parser.cursor)?;
        let v17 = parser.cursor.read_i64_varint()?;

        Ok(json!({
            "AKFKONMJCEC":  string,
            "EGMAFIOOKJJ": v17
        }))
    } else {
        Ok(Value::Null)
    }
}

fn json_enum_parser<'a>(parser: &mut DynamicParser<'a>) -> anyhow::Result<Value> {
    Ok(json!({
        "EnumIndex": parser.cursor.read_i32_varint()?,
        "Value":  parser.cursor.read_i32_varint()?
    }))
}

fn textid_parser<'a>(parser: &mut DynamicParser<'a>) -> anyhow::Result<Value> {
    Ok(json!({
        "Hash": parser.cursor.read_i32_varint()?,
        "Hash64": parser.cursor.read_u64_varint()?
    }))
}
