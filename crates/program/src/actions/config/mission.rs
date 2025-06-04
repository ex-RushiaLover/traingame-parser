use super::{ConfigManifest, parse_config};
use crate::{COUNTER_CONFIGS, parse_and_count};
use anyhow::Result;
use dashmap::DashSet;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
    sync::atomic::Ordering,
};
use tg_parser::DataDefine;

fn read_performance(base_path: &Path, name: &str, out: &mut HashSet<String>) -> Result<()> {
    let entries =
        serde_json::from_slice::<Vec<Value>>(&fs::read(base_path.join(format!("{name}.json")))?)?;

    for item in entries {
        let Some(Value::String(performance_path)) =
            item.get("PerformancePath").or_else(|| item.get("ActPath"))
        else {
            continue;
        };
        out.insert(performance_path.to_string());
    }

    Ok(())
}

/// RPG.GameCore.LevelGraphInfo
fn parse_performances(
    assets: &HashMap<i32, Vec<u8>>,
    types: &HashMap<String, DataDefine>,
    out_folder: &Path,
) {
    let mut performances = HashSet::new();
    let base_path = out_folder.join("ExcelOutput");

    let _ = read_performance(&base_path, "PerformanceA", &mut performances);
    let _ = read_performance(&base_path, "PerformanceC", &mut performances);
    let _ = read_performance(&base_path, "PerformanceCG", &mut performances);
    let _ = read_performance(&base_path, "PerformanceD", &mut performances);
    let _ = read_performance(&base_path, "PerformanceDS", &mut performances);
    let _ = read_performance(&base_path, "PerformanceE", &mut performances);
    let _ = read_performance(&base_path, "PerformanceVideo", &mut performances);
    let _ = read_performance(&base_path, "DialogueNPC", &mut performances);

    performances.par_iter().for_each(|path| {
        parse_and_count!(
            path,
            "RPG.GameCore.LevelGraphConfig",
            assets,
            types,
            out_folder
        )
    });
}

/// RPG.GameCore.MainMissionInfoConfig
fn parse_mission_info(
    assets: &HashMap<i32, Vec<u8>>,
    types: &HashMap<String, DataDefine>,
    out_folder: &Path,
) -> Result<()> {
    let entries = serde_json::from_slice::<Vec<Value>>(&fs::read(
        out_folder.join("ExcelOutput/MainMission.json"),
    )?)?;

    let paths = entries
        .iter()
        .filter_map(|item| {
            if let Some(Value::Number(mission_id)) = item.get("MainMissionID") {
                Some(format!(
                    "Config/Level/Mission/{mission_id}/MissionInfo_{mission_id}.json"
                ))
            } else {
                None
            }
        })
        .collect::<HashSet<_>>();

    let sub_mission_paths = DashSet::new();

    paths.par_iter().for_each(|path| {
        if let Ok(config) = parse_config(
            path,
            "RPG.GameCore.MainMissionInfoConfig",
            assets,
            types,
            out_folder,
        ) {
            COUNTER_CONFIGS.fetch_add(1, Ordering::Relaxed);
            if let Some(Value::Array(sub_mission_list)) = config.get("SubMissionList") {
                for sub_mission in sub_mission_list {
                    let Some(Value::String(json_path)) = sub_mission.get("MissionJsonPath") else {
                        continue;
                    };

                    sub_mission_paths.insert(json_path.to_string());
                }
            }
        }
    });

    sub_mission_paths.par_iter().for_each(|path| {
        parse_and_count!(
            &path,
            "RPG.GameCore.LevelGraphConfig",
            assets,
            types,
            out_folder
        )
    });

    Ok(())
}

pub fn parse(
    assets: &HashMap<i32, Vec<u8>>,
    types: &HashMap<String, DataDefine>,
    out_folder: &Path,
    _: &ConfigManifest,
) -> Result<()> {
    let _ = parse_mission_info(assets, types, out_folder);
    parse_performances(assets, types, out_folder);

    Ok(())
}
