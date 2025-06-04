use super::ConfigManifest;
use crate::parse_and_count;
use anyhow::Result;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde_json::{Map, Value};
use std::{collections::HashMap, fs, path::Path};
use tg_parser::DataDefine;

fn parse_floor(
    assets: &HashMap<i32, Vec<u8>>,
    types: &HashMap<String, DataDefine>,
    out_folder: &Path,
) -> Result<()> {
    let maze_plane: Vec<Map<String, Value>> =
        serde_json::from_slice(&fs::read(out_folder.join("ExcelOutput/MazePlane.json"))?)?;

    // Flatten plane and floor ID tuples
    let paths: Vec<_> = maze_plane
        .iter()
        .flat_map(|p| {
            let list = p
                .get("FloorIDList")
                .unwrap()
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect::<Vec<_>>();
            let plane_id = p.get("PlaneID").unwrap().as_u64().unwrap() as u32;
            list.into_iter().map(move |f| (plane_id, f))
        })
        .collect();

    paths.par_iter().for_each(|(plane_id, floor_id)| {
        let name = format!("P{plane_id}_F{floor_id}");

        let configs = [
            (
                format!("Config/LevelOutput/RuntimeFloor/{name}.json"),
                "RPG.GameCore.RtLevelFloorInfo",
            ),
            (
                format!("Config/LevelOutput_Baked/Floor/{name}_Baked.json"),
                "RPG.GameCore.LevelFloorBakedInfo",
            ),
            (
                format!(
                    "Config/LevelOutput_Baked/FloorCrossMapBriefInfo/CrossMapBriefInfo_{name}.json"
                ),
                "RPG.GameCore.LevelFloorCrossMapBriefInfo",
            ),
            (
                format!("Config/LevelOutput/Region/FloorRegion_{name}.json"),
                "RPG.GameCore.LevelRegionInfos",
            ),
            (
                format!("Config/LevelOutput/RotatableRegion/RotatableRegion_Floor_{floor_id}.json"),
                "RPG.GameCore.MapRotationConfig",
            ),
            (
                format!("Config/LevelOutput/EraFlipper/EraFlipper_Floor_{floor_id}.json"),
                "RPG.GameCore.EraFlipperConfig",
            ),
            (
                format!("Config/LevelOutput/Map/MapInfo_{name}.json"),
                "RPG.GameCore.LevelNavmapConfig",
            ),
        ];

        for (path, type_name) in configs {
            parse_and_count!(&path, type_name, assets, types, out_folder)
        }
    });

    Ok(())
}

fn parse_group(
    assets: &HashMap<i32, Vec<u8>>,
    types: &HashMap<String, DataDefine>,
    out_folder: &Path,
) -> Result<()> {
    let runtime_floor = fs::read_dir(out_folder.join("Config/LevelOutput/RuntimeFloor"))?;

    let mut group_paths = Vec::new();

    for floor in runtime_floor {
        let Ok(entry) = floor else {
            continue;
        };

        let Ok(slice) = fs::read(entry.path()) else {
            continue;
        };

        let json: Value = serde_json::from_slice(&slice)?;
        json.as_object()
            .and_then(|v| v.get("GroupInstanceList")?.as_array())
            .inspect(|arr| {
                group_paths.extend(
                    arr.iter()
                        .filter_map(|item| {
                            item.as_object()
                                .and_then(|v| v.get("GroupPath")?.as_str())
                                .map(|group_path| group_path.to_string())
                        })
                        .collect::<Vec<String>>(),
                );
            });
    }

    group_paths.par_iter().for_each(|path| {
        parse_and_count!(
            path,
            "RPG.GameCore.RtLevelGroupInfoBase",
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
    parse_floor(assets, types, out_folder)?;
    parse_group(assets, types, out_folder)
}
