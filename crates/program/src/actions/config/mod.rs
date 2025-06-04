use anyhow::{Context, Result};
use common::hash;
use rayon::iter::{IntoParallelRefIterator, ParallelBridge as _, ParallelIterator};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs, panic,
    path::{Path, PathBuf},
};
use tg_parser::{DataDefine, DynamicParser, ValueKind};

mod adventure_ability;
mod adventure_modifier;
mod complex_skill_ai_global;
mod config_ability;
mod global_modifier;
mod global_task_template;
mod level_output;
mod mission;
mod rogue_chest_map;
mod rogue_npc;
mod skill_tree_point_preset;
mod summon_unit;
mod video_caption;

#[macro_export]
macro_rules! parse_and_count {
    ($path:expr, $type:expr, $assets:expr, $types:expr, $out_folder:expr) => {
        if $crate::actions::config::parse_config($path, $type, $assets, $types, $out_folder).is_ok()
        {
            $crate::COUNTER_CONFIGS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    };
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ConfigManifest {
    adventure_ability_config: Vec<String>,
    turn_based_ability_config: Vec<String>,
    // battle_lineup_config: Vec<String>,
    // battle_lineup_avatar_config: Vec<String>,
    // battle_lineup_maze_buff_config: Vec<String>,
    battle_lineup_skill_tree_preset_config: Vec<String>,
    // #[serde(rename = "BattleLineupCEPresetConfig")]
    // battle_lineup_cepreset_config: Vec<String>,
    global_modifier_config: Vec<String>,
    adventure_modifier_config: Vec<String>,
    #[serde(rename = "ComplexSkillAIGlobalGroupConfig")]
    complex_skill_aiglobal_group_config: Vec<String>,
    global_task_template: Vec<String>,
    // common_skill_pool_config: Vec<String>,
}

#[inline]
fn split_path(path: &str) -> Option<(String, String)> {
    path.rsplit_once('/').map(|(dir, file)| {
        (
            if dir.is_empty() { "/" } else { dir }.to_string(),
            file.to_string(),
        )
    })
}

fn parse_config(
    json_path: &str,
    type_name: &str,
    assets: &HashMap<i32, Vec<u8>>,
    types: &HashMap<String, DataDefine>,
    out_folder: &Path,
) -> Result<Value> {
    let (folder_path, file_name) = split_path(json_path).context("Invalid path")?;
    let path_hash = hash::get_32bit_hash_const(&format!(
        "BakedConfig/{}",
        json_path.replace(".json", ".bytes")
    ));

    let bytes = assets.get(&path_hash).ok_or_else(|| {
        // tracing::debug!("Asset not found: {json_path} ({path_hash})");
        anyhow::anyhow!("Asset not found")
    })?;

    match panic::catch_unwind(|| {
        let mut parser = DynamicParser::new(types, bytes);
        parser.parse(&ValueKind::Class(type_name.to_string()), false)
    }) {
        Ok(Ok(parsed)) => {
            let out_folder = out_folder.join(folder_path);
            fs::create_dir_all(&out_folder)?;
            let out_path = out_folder.join(file_name);
            fs::write(&out_path, serde_json::to_string_pretty(&parsed)?)
                .context(format!("Failed to write to {out_path:?}"))?;
            return Ok(parsed);
        }
        Ok(Err(err)) => tracing::error!("Parse error for {json_path} ({type_name}): {err:?}"),
        Err(err) => tracing::error!("Panic during parsing {json_path} ({type_name}): {err:?}"),
    }

    Ok(json!({}))
}

type ParseFn =
    fn(&HashMap<i32, Vec<u8>>, &HashMap<String, DataDefine>, &Path, &ConfigManifest) -> Result<()>;

pub fn parse_configs(
    assets: &HashMap<i32, Vec<u8>>,
    types: &HashMap<String, DataDefine>,
    out_folder: &Path,
    additional_paths: Option<PathBuf>,
) -> Result<()> {
    tracing::info!("Parsing Configs...");
    let config_manifest_bytes = assets
        .get(&hash::get_32bit_hash_const(
            "BakedConfig/ConfigManifest.json",
        ))
        .ok_or_else(|| anyhow::anyhow!("ConfigManifest.json not found"))?;
    let config_manifest: ConfigManifest =
        serde_json::from_slice(config_manifest_bytes).context("Failed to parse ConfigManifest")?;

    let parses: &[(ParseFn, &str)] = &[
        (adventure_ability::parse, "adventure_ability"),
        (config_ability::parse, "config_ability"),
        (global_modifier::parse, "global_modifier"),
        (skill_tree_point_preset::parse, "skill_tree_point_preset"),
        (adventure_modifier::parse, "adventure_modifier"),
        (complex_skill_ai_global::parse, "complex_skill_ai_global"),
        (global_task_template::parse, "global_task_template"),
        (level_output::parse, "level_output"),
        (summon_unit::parse, "summon_unit"),
        (mission::parse, "mission"),
        (video_caption::parse, "video_caption"),
        (rogue_npc::parse, "rogue_npc"),
        (rogue_chest_map::parse, "rogue_chest_map"),
    ];

    parses.par_iter().for_each(|(parse_fn, name)| {
        if let Err(err) =
            panic::catch_unwind(|| parse_fn(assets, types, out_folder, &config_manifest).unwrap())
        {
            tracing::error!("Failed to parse {}: {:?}", name, err);
        }
    });

    // Parse additional paths if provided
    additional_paths
        .and_then(|path| {
            std::fs::read(path).ok().and_then(|bytes| {
                serde_json::from_slice::<HashMap<String, Vec<String>>>(&bytes).ok()
            })
        })
        .unwrap_or_default()
        .iter()
        .flat_map(|(data_type, paths)| paths.iter().map(move |json_path| (json_path, data_type)))
        .par_bridge()
        .for_each(|(json_path, data_type)| {
            parse_and_count!(json_path, data_type, assets, types, out_folder);
        });

    Ok(())
}
