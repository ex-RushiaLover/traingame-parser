use super::hash;
use anyhow::{Context, Result};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::{StatusCode, blocking::Client};
use std::{
    collections::HashMap,
    io::{Cursor, Read as _},
    path::PathBuf,
    thread,
    time::Duration,
};
use tg_asset_meta::{
    design_index::{DesignIndex, FileEntry},
    mini_asset::MiniAsset,
};
use tg_bytes_util::FromBytes;

const MAX_RETRIES: usize = 3;
const RETRY_DELAY: Duration = Duration::from_millis(100);

pub fn download_all_design_data(
    design_data_url: String,
    output_folder: Option<PathBuf>,
    filter_hashes: Vec<i32>,
) -> Result<HashMap<i32, Vec<u8>>> {
    let client = Client::new();
    let mp = MultiProgress::new();

    let mini_asset = download_mini_asset(&client, &design_data_url, &mp, &output_folder)
        .context("Failed to download mini asset")?;

    let design_index =
        download_design_index(&client, &design_data_url, &mp, &mini_asset, &output_folder)
            .context("Failed to download design index")?;

    let mut handles = HashMap::with_capacity(design_index.file_list.len());

    for file_entry in design_index.file_list {
        let client = client.clone();
        let design_data_url = design_data_url.clone();
        let mp = mp.clone();
        let output_folder = output_folder.clone();
        let byte_name = file_entry.file_byte_name.clone();

        if !filter_hashes.is_empty()
            && !file_entry
                .data_entries
                .iter()
                .any(|e| filter_hashes.contains(&e.name_hash))
        {
            continue;
        }

        handles.insert(
            byte_name,
            thread::spawn(move || {
                let data = download_design_bytes(
                    &client,
                    &design_data_url,
                    &mp,
                    &file_entry,
                    &output_folder,
                )?;

                // Special handling for ConfigManifest, since they are in JSON format.
                if file_entry.name_hash
                    == hash::get_32bit_hash_const("BakedConfig/ConfigManifest.json")
                {
                    return Result::<HashMap<i32, Vec<u8>>>::Ok(HashMap::from([(
                        file_entry.name_hash,
                        data,
                    )]));
                };

                Ok(file_entry
                    .data_entries
                    .iter()
                    .map(|data_entry| {
                        let slice = &data[data_entry.offset as usize
                            ..(data_entry.offset + data_entry.size) as usize];

                        (data_entry.name_hash, slice.to_vec())
                    })
                    .collect::<HashMap<i32, Vec<u8>>>())
            }),
        );
    }

    let results: HashMap<String, HashMap<i32, Vec<u8>>> = handles
        .into_iter()
        .filter_map(|(byte_name, handle)| match handle.join() {
            Ok(Ok(data)) => Some((byte_name, data)),
            Ok(Err(e)) => {
                tracing::error!("Download error: {:?}", e);
                None
            }
            Err(e) => {
                tracing::error!("Thread panicked: {:?}", e);
                None
            }
        })
        .collect();

    Ok(results
        .into_iter()
        .flat_map(|(_, inner)| inner.into_iter())
        .collect())
}

#[inline]
fn download_mini_asset(
    client: &Client,
    design_data_url: &str,
    mp: &MultiProgress,
    output_folder: &Option<PathBuf>,
) -> anyhow::Result<MiniAsset> {
    let res = download_bytes(
        client,
        &format!("{design_data_url}/client/Windows/M_DesignV.bytes"),
        mp,
    )?;
    let mini_asset = MiniAsset::from_bytes(&mut Cursor::new(&res))?;

    save_file(output_folder, &res, "M_DesignV.bytes");

    Ok(mini_asset)
}

#[inline]
fn download_design_index(
    client: &Client,
    design_data_url: &str,
    mp: &MultiProgress,
    mini_asset: &MiniAsset,
    output_folder: &Option<PathBuf>,
) -> Result<DesignIndex> {
    let name = format!("DesignV_{}.bytes", mini_asset.design_index_hash);
    let res = download_bytes(
        client,
        &format!("{design_data_url}/client/Windows/{name}",),
        mp,
    )?;

    let design_index = DesignIndex::from_bytes(&mut Cursor::new(&res))?;

    save_file(output_folder, &res, &name);

    Ok(design_index)
}

#[inline]
fn download_design_bytes(
    client: &Client,
    design_data_url: &str,
    mp: &MultiProgress,
    file_entry: &FileEntry,
    output_folder: &Option<PathBuf>,
) -> Result<Vec<u8>> {
    let name = format!("{}.bytes", file_entry.file_byte_name);
    let bytes = download_bytes(
        client,
        &format!("{design_data_url}/client/Windows/{name}",),
        mp,
    )?;
    save_file(output_folder, &bytes, &name);

    Ok(bytes)
}

#[inline]
fn save_file(output_folder: &Option<PathBuf>, bytes: &Vec<u8>, file_name: &str) {
    if let Some(output_folder) = output_folder {
        let output_folder = output_folder.join("DesignData");
        if !output_folder.is_dir() {
            let _ = std::fs::create_dir_all(&output_folder);
        }
        let _ = std::fs::write(output_folder.join(file_name), bytes);
    }
}

fn download_bytes(client: &Client, design_data_url: &str, mp: &MultiProgress) -> Result<Vec<u8>> {
    if !design_data_url.starts_with("http") {
        return Ok(std::fs::read(
            design_data_url.replace("client/Windows", ""),
        )?);
    }

    for attempt in 1..=MAX_RETRIES {
        let result = (|| -> Result<Vec<u8>> {
            let resp = client.get(design_data_url).send()?;
            let status = resp.status();
            if status != StatusCode::OK {
                return Err(anyhow::format_err!(
                    "Server returned non OK code for {design_data_url} {:?}",
                    status
                ));
            }

            let total = resp.content_length().unwrap_or(0);
            let pb = mp.add(ProgressBar::new(total));
            pb.set_style(
                ProgressStyle::with_template("{msg} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, ETA: {eta})")?
                    .progress_chars("##-"),
            );

            let file_name = design_data_url.split('/').next_back().unwrap_or_default();
            pb.set_message(format!("Downloading {file_name}"));

            let mut reader = pb.wrap_read(resp);
            let mut buffer = Vec::with_capacity(total as usize);
            reader.read_to_end(&mut buffer)?;

            pb.finish_with_message(format!("Downloaded {file_name}"));
            Ok(buffer)
        })();

        match result {
            Ok(data) => return Ok(data),
            Err(e) if attempt < MAX_RETRIES => {
                mp.println(format!(
                    "Retry {attempt}/{MAX_RETRIES} for {design_data_url} due to error: {e}"
                ))?;
                std::thread::sleep(RETRY_DELAY);
            }
            Err(e) => return Err(e),
        }
    }

    unreachable!()
}
