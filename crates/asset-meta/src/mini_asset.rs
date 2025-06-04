use std::io::SeekFrom;

use byteorder::{LE, ReadBytesExt};
use tg_bytes_util::{ByteHash16, FromBytes};

#[derive(Debug)]
pub struct MiniAsset {
    pub revision_id: u32,
    pub design_index_hash: ByteHash16,
}

impl FromBytes for MiniAsset {
    fn from_bytes<T: std::io::Seek + std::io::Read>(r: &mut T) -> std::io::Result<Self> {
        r.seek(SeekFrom::Current(6 * 4))?;
        Ok(Self {
            revision_id: r.read_u32::<LE>()?,
            design_index_hash: ByteHash16::from_bytes(r)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::mini_asset::MiniAsset;
    use std::io::Cursor;
    use tg_bytes_util::FromBytes;

    #[test]
    fn test() {
        const BYTES: &[u8] = &[
            83, 82, 77, 73, 0, 3, 0, 1, 66, 0, 0, 0, 0, 0, 12, 0, 3, 0, 0, 0, 2, 0, 0, 0, 234, 255,
            151, 0, 202, 110, 28, 223, 138, 63, 212, 4, 63, 130, 138, 178, 68, 22, 219, 131, 234,
            55, 0, 0, 0, 0, 0, 0, 210, 249, 237, 103, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let mut r = Cursor::new(BYTES);
        let parsed = MiniAsset::from_bytes(&mut r).unwrap();
        assert_eq!(
            parsed.design_index_hash.to_string(),
            "df1c6eca04d43f8ab28a823f83db1644"
        )
    }
}
