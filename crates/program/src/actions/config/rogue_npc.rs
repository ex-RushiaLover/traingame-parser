use super::ConfigManifest;
use crate::{COUNTER_CONFIGS, actions::config::parse_config, parse_and_count};
use anyhow::Result;
use dashmap::DashSet;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde_json::Value;
use std::{collections::HashMap, fs, path::Path, sync::atomic::Ordering};
use tg_parser::DataDefine;

#[inline]
fn extract_npc_json_paths<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<String>> {
    let data = fs::read(path)?;
    let json: Vec<Value> = serde_json::from_slice(&data)?;
    Ok(json
        .into_iter()
        .filter_map(|item| item.get("NPCJsonPath")?.as_str().map(|s| s.to_string()))
        .collect())
}

/// RPG.GameCore.RogueNPCConfig
pub fn parse(
    assets: &HashMap<i32, Vec<u8>>,
    types: &HashMap<String, DataDefine>,
    out_folder: &Path,
    _: &ConfigManifest,
) -> Result<()> {
    let paths = [
        "ExcelOutput/RogueNPC.json",
        "ExcelOutput/RogueTournNPC.json",
        "ExcelOutput/RogueMagicNPC.json",
    ]
    .iter()
    .filter_map(|path| extract_npc_json_paths(out_folder.join(path)).ok())
    .flatten()
    .collect::<Vec<_>>();

    let dialogue_paths = DashSet::new();
    let option_paths = DashSet::new();

    paths.par_iter().for_each(|path| {
        if let Ok(config) = parse_config(
            path,
            "RPG.GameCore.RogueNPCConfig",
            assets,
            types,
            out_folder,
        ) {
            COUNTER_CONFIGS.fetch_add(1, Ordering::Relaxed);
            if let Some(Value::Array(dialogue_list)) = config.get("DialogueList") {
                for dialogue in dialogue_list {
                    if let Some(Value::String(json_path)) = dialogue.get("DialoguePath") {
                        dialogue_paths.insert(json_path.to_string());
                    };

                    if let Some(Value::String(json_path)) = dialogue.get("OptionPath") {
                        option_paths.insert(json_path.to_string());
                    };
                }
            }
        }
    });

    dialogue_paths.par_iter().for_each(|path| {
        parse_and_count!(
            &path,
            "RPG.GameCore.LevelGraphConfig",
            assets,
            types,
            out_folder
        )
    });

    option_paths.par_iter().for_each(|path| {
        parse_and_count!(
            &path,
            "RPG.GameCore.RogueDialogueEventConfig",
            assets,
            types,
            out_folder
        )
    });

    Ok(())
}
