# Traingame parser

A tool to parse textmap (and maybe other res too) for a certain anime game.

---

## Usages

### `textmap`

Processes **only** the textmap files.

```bash
./program.exe textmap <Persistent Path / Design Data URL> <OUTPUT_DIR> [OPTIONS]
```

**Arguments:**

- `input_url` — URL or path to the persistent data or design bundle
- `output_dir` — Directory where the parsed output will be stored

**Options:**

- `--full-textmap` — Parse the entire textmap structure as an array instead of just key-value pairs
- `--save-bytes-file` — Save the `.bytes` files after download

**Examples:**

```bash
./program.exe textmap "https://autopatchcn.bhsr.com/design_data/BetaLive/output_10494861_2ed49bac2846_b7f8d02fced269" output/
```

```bash
./program.exe textmap "D:/Star Rail/StarRail_Data/Persistent/DesignData/Windows" output/
```

<details>
<summary><strong><code>excels</code></strong></summary>

### `excels`

Processes the Excel & Textmaps files

```bash
./program.exe excels <DATA_JSON> <EXCEL_PATH_JSON> <Persistent Path / Design Data URL> <OUTPUT_DIR> [OPTIONS]
```

**Arguments:**

- `data_json` — Path to `data.json` schema
- `excel_path_json` — JSON file that maps Excel types to file paths
- `input_url` — URL or path to the persistent data
- `output_dir` — Output folder for processed files

**Options:**

- `--full-textmap` — Enable full textmap parsing if needed for linked data
- `--save-bytes-file` — Save original `.bytes` files
- `--log-error` — Output all encountered errors to the console
- `--config-paths <PATH>` — Optional extra config files (in JSON) for parsing additional types

**Examples:**

```bash
./program.exe excels data.json excels_path.json https://autopatchcn.bhsr.com/design_data/BetaLive/output_10494861_2ed49bac2846_b7f8d02fced269 output/ --log-error --save-bytes-file
```

```bash
./program.exe excels data.json excels_path.json "D:/Star Rail/StarRail_Data/Persistent/DesignData/Windows" output/ --log-error --save-bytes-file
```

</details>

<details>
<summary><strong><code>all</code></strong></summary>

### `all`

Processes Textmap, Excels, and Config files

```bash
./program.exe all <DATA_JSON> <EXCEL_PATH_JSON> <Persistent Path / Design Data URL> <OUTPUT_DIR> [OPTIONS]
```

Accepts the **same arguments and options** as the `excels` command.

**Examples:**

```bash
./program.exe all data.json excels_path.json "https://autopatchcn.bhsr.com/design_data/BetaLive/output_10494861_2ed49bac2846_b7f8d02fced269" output/ --full-textmap --log-error
```

```bash
./program.exe all data.json excels_path.json "D:/Star Rail/StarRail_Data/Persistent/DesignData/Windows" output/ --full-textmap --log-error
```

</details>

## Notes

- For parsing anything other than textmap (i.e., `excels` or `all`), **you must generate `data.json` and `excels_path.json` yourself**.

---

## Credits / References

- https://arca.live/b/starrailleaks/76183295
- https://github.com/Hiro420/HSR_Downloader
