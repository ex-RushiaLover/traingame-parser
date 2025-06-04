use crate::{
    actions::textmap::TEXTMAP_PATHS,
    cli::{Cli, Command},
};
use anyhow::{Context as _, Result};
use clap::Parser;
use common::downloader;
use std::{
    collections::HashMap,
    fs,
    sync::atomic::{AtomicI32, Ordering},
    time::Instant,
};
use tg_parser::DataDefine;
use tracing::Level;

mod actions;
mod cli;

pub static COUNTER_CONFIGS: AtomicI32 = AtomicI32::new(0);
pub static COUNTER_EXCELS: AtomicI32 = AtomicI32::new(0);
pub static COUNTER_TEXTMAPS: AtomicI32 = AtomicI32::new(0);

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Command::Textmap {
            input_url,
            output_dir,
            full_textmap,
            save_bytes_file,
        } => {
            common::logging::init(Level::INFO);

            let start = Instant::now();

            let assets = downloader::download_all_design_data(
                input_url.clone(),
                if *save_bytes_file {
                    Some(output_dir.clone())
                } else {
                    None
                },
                TEXTMAP_PATHS.iter().map(|v| v.1).collect(),
            )?;

            tracing::info!("Download Done! Took {}s", start.elapsed().as_secs());

            let start = Instant::now();

            actions::textmap::parse_all_textmap(&assets, output_dir, !full_textmap)?;

            tracing::info!("Textmap Parse Done! Took {}ms", start.elapsed().as_millis());
        }

        Command::Excels(args) | Command::All(args) => {
            if args.log_error {
                common::logging::init(Level::INFO)
            } else {
                common::logging::init_info_only();
            }

            let assets = downloader::download_all_design_data(
                args.input_url.clone(),
                if args.save_bytes_file {
                    Some(args.output_dir.clone())
                } else {
                    None
                },
                Vec::with_capacity(0),
            )?;

            let start = Instant::now();

            let excel_paths: HashMap<String, Vec<String>> = serde_json::from_slice(
                &fs::read(&args.excel_path_json).context("Failed to read excel_paths.json")?,
            )?;

            let types: HashMap<String, DataDefine> = serde_json::from_slice(
                &fs::read(&args.data_json).context("Failed to read data.json")?,
            )?;

            actions::excel::parse_all_excels(
                &assets,
                &types,
                &args.output_dir.clone(),
                &excel_paths,
            )?;

            if let Command::All(_) = cli.command {
                actions::config::parse_configs(
                    &assets,
                    &types,
                    &args.output_dir,
                    args.config_paths.clone(),
                )?;
                actions::textmap::parse_all_textmap(&assets, &args.output_dir, !args.full_textmap)?;
            }

            tracing::info!(
                "Parsed {} Excels, {} Configs, and {} Textmaps in {}s",
                COUNTER_EXCELS.load(Ordering::Relaxed),
                COUNTER_CONFIGS.load(Ordering::Relaxed),
                COUNTER_TEXTMAPS.load(Ordering::Relaxed),
                start.elapsed().as_secs()
            );
        }
    }

    Ok(())
}

// fn main() {
//     use std::collections::HashMap;
//     use tg_parser::{DynamicParser, ValueKind};
//     let assets = common::downloader::download_all_design_data(
//         String::from(
//             "C:/Data/hoyoreverse/StarRail_3.3.51/StarRail_Data/Persistent/DesignData/Windows",
//         ),
//         None,
//     )
//     .unwrap();
//     common::logging::init(tracing::Level::DEBUG);
//     let bytes = assets
//         .get(&common::hash::get_32bit_hash(
//             "BakedConfig/Config/AudioConfig.bytes",
//         ))
//         .unwrap();
//     let bytes = &bytes[12..].to_vec();
//     let schema: HashMap<String, DataDefine> =
//         serde_json::from_slice(&std::fs::read("data.json").unwrap()).unwrap();
//     let mut parser = DynamicParser::new(&schema, &bytes);
//     let parsed = parser
//         .parse(
//             &ValueKind::Class(String::from("RPG.GameCore.AudioConfig")),
//             false,
//         )
//         .unwrap();
//     std::fs::write("ss.json", serde_json::to_string_pretty(&parsed).unwrap()).unwrap();
// }
