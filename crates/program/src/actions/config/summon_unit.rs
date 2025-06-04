use super::ConfigManifest;
use crate::parse_and_count;
use anyhow::Result;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde_json::Value;
use std::{collections::HashMap, fs, path::Path};
use tg_parser::DataDefine;

/// RPG.GameCore.SummonUnitConfig
pub fn parse(
    assets: &HashMap<i32, Vec<u8>>,
    types: &HashMap<String, DataDefine>,
    out_folder: &Path,
    _: &ConfigManifest,
) -> Result<()> {
    let summon_unit: Vec<Value> = serde_json::from_slice(&fs::read(
        out_folder.join("ExcelOutput/SummonUnitData.json"),
    )?)?;

    summon_unit.par_iter().for_each(|summon_unit| {
        parse_and_count!(
            summon_unit.get("JsonPath").unwrap().as_str().unwrap(),
            "RPG.GameCore.SummonUnitConfig",
            assets,
            types,
            out_folder
        )
    });

    Ok(())
}
