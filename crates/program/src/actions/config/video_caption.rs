use super::ConfigManifest;
use crate::parse_and_count;
use anyhow::Result;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde_json::Value;
use std::{collections::HashMap, fs, path::Path};
use tg_parser::DataDefine;

#[inline]
fn extract_caption_paths<P: AsRef<Path>>(path: P) -> std::io::Result<Vec<String>> {
    let data = fs::read(path)?;
    let json: Vec<Value> = serde_json::from_slice(&data)?;
    Ok(json
        .into_iter()
        .filter_map(|item| item.get("CaptionPath")?.as_str().map(|s| s.to_string()))
        .collect())
}

/// RPG.GameCore.VideoCaptionConfig
pub fn parse(
    assets: &HashMap<i32, Vec<u8>>,
    types: &HashMap<String, DataDefine>,
    out_folder: &Path,
    _: &ConfigManifest,
) -> Result<()> {
    let paths = [
        "ExcelOutput/VideoConfig.json",
        "ExcelOutput/CutSceneConfig.json",
        "ExcelOutput/LoopCGConfig.json",
    ]
    .iter()
    .filter_map(|path| extract_caption_paths(out_folder.join(path)).ok())
    .flatten()
    .collect::<Vec<_>>();

    paths
        .par_iter()
        .filter(|path| !path.is_empty())
        .for_each(|path| {
            parse_and_count!(
                path,
                "RPG.GameCore.VideoCaptionConfig",
                assets,
                types,
                out_folder
            )
        });

    Ok(())
}
