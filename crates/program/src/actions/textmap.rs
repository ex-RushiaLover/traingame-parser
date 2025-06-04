use std::{collections::HashMap, io::Cursor, path::Path, sync::atomic::Ordering};

use anyhow::Result;
use common::hash::get_32bit_hash_const;
use serde::Serialize;
use serde_json::{Map, Value};
use tg_bytes_util::{ExistFlag, FromBytes};

use crate::COUNTER_TEXTMAPS;

pub const TEXTMAP_PATHS: [(&str, i32); 28] = [
    (
        "TextMapEN.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/Textmap_en.bytes"),
    ),
    (
        "TextMapCN.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/Textmap_cn.bytes"),
    ),
    (
        "TextMapKR.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/Textmap_kr.bytes"),
    ),
    (
        "TextMapJP.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/Textmap_jp.bytes"),
    ),
    (
        "TextMapID.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/Textmap_id.bytes"),
    ),
    (
        "TextMapCHS.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/Textmap_chs.bytes"),
    ),
    (
        "TextMapCHT.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/Textmap_cht.bytes"),
    ),
    (
        "TextMapDE.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/Textmap_de.bytes"),
    ),
    (
        "TextMapES.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/Textmap_es.bytes"),
    ),
    (
        "TextMapFR.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/Textmap_fr.bytes"),
    ),
    (
        "TextMapRU.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/Textmap_ru.bytes"),
    ),
    (
        "TextMapTH.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/Textmap_th.bytes"),
    ),
    (
        "TextMapVI.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/Textmap_vi.bytes"),
    ),
    (
        "TextMapPT.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/Textmap_pt.bytes"),
    ),
    (
        "TextMapMainEN.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/TextmapMain_en.bytes"),
    ),
    (
        "TextMapMainCN.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/TextmapMain_cn.bytes"),
    ),
    (
        "TextMapMainKR.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/TextmapMain_kr.bytes"),
    ),
    (
        "TextMapMainJP.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/TextmapMain_jp.bytes"),
    ),
    (
        "TextMapMainID.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/TextmapMain_id.bytes"),
    ),
    (
        "TextMapMainCHS.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/TextmapMain_chs.bytes"),
    ),
    (
        "TextMapMainCHT.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/TextmapMain_cht.bytes"),
    ),
    (
        "TextMapMainDE.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/TextmapMain_de.bytes"),
    ),
    (
        "TextMapMainES.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/TextmapMain_es.bytes"),
    ),
    (
        "TextMapMainFR.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/TextmapMain_fr.bytes"),
    ),
    (
        "TextMapMainRU.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/TextmapMain_ru.bytes"),
    ),
    (
        "TextMapMainTH.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/TextmapMain_th.bytes"),
    ),
    (
        "TextMapMainVI.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/TextmapMain_vi.bytes"),
    ),
    (
        "TextMapMainPT.json",
        get_32bit_hash_const("BakedConfig/ExcelOutput/TextmapMain_pt.bytes"),
    ),
];

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct TextID {
    pub hash: i32,
    pub hash_64: u64,
}

impl FromBytes for TextID {
    fn from_bytes<T: std::io::Seek + std::io::Read>(r: &mut T) -> std::io::Result<Self> {
        Ok(Self {
            hash: i32::from_bytes(r)?,
            hash_64: u64::from_bytes(r)?,
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "PascalCase")]
struct TextMapRow {
    #[serde(rename = "ID")]
    pub id: Option<TextID>,
    pub text: String,
    pub has_param: bool,
}

impl FromBytes for TextMapRow {
    fn from_bytes<T: std::io::Seek + std::io::Read>(r: &mut T) -> std::io::Result<Self> {
        let exist_flag = ExistFlag::new(r, 3)?;
        Ok(Self {
            id: if exist_flag.exists(0) {
                Some(TextID::from_bytes(r)?)
            } else {
                None
            },
            text: if exist_flag.exists(1) {
                String::from_bytes(r)?
            } else {
                String::with_capacity(0)
            },
            has_param: if exist_flag.exists(2) {
                bool::from_bytes(r)?
            } else {
                false
            },
        })
    }
}

pub fn parse_all_textmap(
    assets: &HashMap<i32, Vec<u8>>,
    out_folder: &Path,
    minimal: bool,
) -> Result<()> {
    tracing::info!("Parsing Textmaps...");

    let out_folder = out_folder.join("TextMap");

    if !out_folder.exists() {
        std::fs::create_dir_all(&out_folder)?;
    }

    for (name, hash) in TEXTMAP_PATHS {
        let Some(asset) = assets.get(&hash) else {
            continue;
        };

        // Skip empty first byte
        let asset = if !asset.is_empty() && asset[0] == 0 {
            &asset[1..]
        } else {
            asset
        };

        let out_path = out_folder.join(name);
        let mut cursor = Cursor::new(asset);

        let Ok(parsed) = Vec::<TextMapRow>::from_bytes(&mut cursor) else {
            continue;
        };

        if minimal {
            std::fs::write(
                out_path,
                serde_json::to_string_pretty(
                    &parsed
                        .into_iter()
                        .map(|row| {
                            (
                                row.id.map(|v| v.hash).unwrap_or_default().to_string(),
                                Value::String(row.text),
                            )
                        })
                        .collect::<Map<_, _>>(),
                )
                .unwrap(),
            )
            .unwrap();
            COUNTER_TEXTMAPS.fetch_add(1, Ordering::Relaxed);
        } else {
            std::fs::write(out_path, serde_json::to_string_pretty(&parsed).unwrap()).unwrap();
        }
    }

    Ok(())
}
