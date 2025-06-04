use byteorder::{BE, ReadBytesExt};
use std::fmt::Write;
use tg_bytes_util::FromBytes;

#[derive(Debug)]
pub struct DesignIndex {
    pub unk_i64: i64,
    pub file_count: i32,
    pub design_data_count: i32,
    pub file_list: Vec<FileEntry>,
}

impl FromBytes for DesignIndex {
    fn from_bytes<T: std::io::Seek + std::io::Read>(r: &mut T) -> std::io::Result<Self> {
        let mut result = DesignIndex {
            unk_i64: r.read_i64::<BE>()?,
            file_count: r.read_i32::<BE>()?,
            design_data_count: r.read_i32::<BE>()?,
            file_list: vec![],
        };

        for _ in 0..result.file_count {
            result.file_list.push(FileEntry::from_bytes(r)?);
        }

        Ok(result)
    }
}

#[derive(Debug)]
pub struct FileEntry {
    pub name_hash: i32,
    pub file_byte_name: String,
    pub size: i64,
    pub data_count: i32,
    pub data_entries: Vec<DataEntry>,
    pub unk: u8,
}

impl FromBytes for FileEntry {
    fn from_bytes<T: std::io::Seek + std::io::Read>(r: &mut T) -> std::io::Result<Self> {
        let mut result = Self {
            name_hash: r.read_i32::<BE>()?,
            file_byte_name: {
                let mut buf = vec![0u8; 16];
                r.read_exact(&mut buf)?;
                buf.iter().fold(String::with_capacity(16), |mut output, b| {
                    let _ = output.write_str(&format!("{b:02x}"));
                    output
                })
            },
            size: r.read_i64::<BE>()?,
            data_count: r.read_i32::<BE>()?,
            data_entries: vec![],
            unk: 0,
        };

        for _ in 0..result.data_count {
            result.data_entries.push(DataEntry::from_bytes(r)?);
        }

        result.unk = r.read_u8()?;

        Ok(result)
    }
}

#[derive(Debug)]
pub struct DataEntry {
    pub name_hash: i32,
    pub size: u32,
    pub offset: u32,
}

impl FromBytes for DataEntry {
    fn from_bytes<T: std::io::Seek + std::io::Read>(r: &mut T) -> std::io::Result<Self> {
        Ok(Self {
            name_hash: r.read_i32::<BE>()?,
            size: r.read_u32::<BE>()?,
            offset: r.read_u32::<BE>()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::design_index::DesignIndex;
    use std::io::Cursor;
    use tg_bytes_util::FromBytes;

    #[test]
    fn test_parse_design_index() {
        const BYTES: &[u8] = include_bytes!("../tests/DesignV.bytes");

        let mut r = Cursor::new(BYTES);
        let parsed = DesignIndex::from_bytes(&mut r).unwrap();

        assert_eq!(11, parsed.file_count);
        assert_eq!(100102, parsed.design_data_count);
        assert_eq!(11, parsed.file_list.len());

        // FileEntry
        assert_eq!(-1703948225, parsed.file_list[0].name_hash);
        assert_eq!(
            "7e3fc08e24890ba15f9c3a8ec1454025",
            parsed.file_list[0].file_byte_name.to_string()
        );
        assert_eq!(89899, parsed.file_list[0].size);
        assert_eq!(1, parsed.file_list[0].data_count);

        // DataEntry
        assert_eq!(-1703948225, parsed.file_list[0].data_entries[0].name_hash);
        assert_eq!(89899, parsed.file_list[0].data_entries[0].size);
        assert_eq!(0, parsed.file_list[0].data_entries[0].offset);
    }
}
