use super::ConfigManifest;
use crate::parse_and_count;
use anyhow::Result;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::{collections::HashMap, path::Path};
use tg_parser::DataDefine;

/// RPG.GameCore.AdventureAbilityConfigList
pub fn parse(
    assets: &HashMap<i32, Vec<u8>>,
    types: &HashMap<String, DataDefine>,
    out_folder: &Path,
    config_manifest: &ConfigManifest,
) -> Result<()> {
    config_manifest
        .adventure_ability_config
        .par_iter()
        .for_each(|json_path| {
            parse_and_count!(
                json_path,
                "RPG.GameCore.AdventureAbilityConfigList",
                assets,
                types,
                out_folder
            );
        });

    Ok(())
}
