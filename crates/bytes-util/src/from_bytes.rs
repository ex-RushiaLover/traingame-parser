use std::{
    collections::HashMap,
    hash::Hash,
    io::{self, Read, Seek},
};
use varint_rs::VarintReader;

pub trait FromBytes: Send + Sync + Sized {
    fn from_bytes<T: io::Seek + io::Read>(r: &mut T) -> io::Result<Self>;
}

impl FromBytes for u8 {
    #[inline]
    fn from_bytes<T: Seek + Read>(r: &mut T) -> io::Result<Self> {
        r.read_u8_varint()
    }
}

impl FromBytes for u16 {
    #[inline]
    fn from_bytes<T: Seek + Read>(r: &mut T) -> io::Result<Self> {
        r.read_u16_varint()
    }
}

impl FromBytes for u32 {
    #[inline]
    fn from_bytes<T: Seek + Read>(r: &mut T) -> io::Result<Self> {
        r.read_u32_varint()
    }
}

impl FromBytes for u64 {
    #[inline]
    fn from_bytes<T: Seek + Read>(r: &mut T) -> io::Result<Self> {
        r.read_u64_varint()
    }
}

impl FromBytes for usize {
    #[inline]
    fn from_bytes<T: Seek + Read>(r: &mut T) -> io::Result<Self> {
        r.read_usize_varint()
    }
}

impl FromBytes for i8 {
    #[inline]
    fn from_bytes<T: Seek + Read>(r: &mut T) -> io::Result<Self> {
        r.read_i8_varint()
    }
}

impl FromBytes for i16 {
    #[inline]
    fn from_bytes<T: Seek + Read>(r: &mut T) -> io::Result<Self> {
        r.read_i16_varint()
    }
}

impl FromBytes for i32 {
    #[inline]
    fn from_bytes<T: Seek + Read>(r: &mut T) -> io::Result<Self> {
        r.read_i32_varint()
    }
}

impl FromBytes for i64 {
    #[inline]
    fn from_bytes<T: Seek + Read>(r: &mut T) -> io::Result<Self> {
        r.read_i64_varint()
    }
}

impl FromBytes for isize {
    #[inline]
    fn from_bytes<T: Seek + Read>(r: &mut T) -> io::Result<Self> {
        r.read_isize_varint()
    }
}

impl FromBytes for bool {
    #[inline]
    fn from_bytes<T: Seek + Read>(r: &mut T) -> io::Result<Self> {
        Ok(r.read_i8_varint()? != 0)
    }
}

impl FromBytes for f32 {
    #[inline]
    fn from_bytes<T: Seek + Read>(r: &mut T) -> io::Result<Self> {
        let mut byte = [0; 4];
        r.read_exact(&mut byte)?;
        Ok(f32::from_le_bytes(byte))
    }
}

impl FromBytes for f64 {
    #[inline]
    fn from_bytes<T: Seek + Read>(r: &mut T) -> io::Result<Self> {
        let mut byte = [0; 8];
        r.read_exact(&mut byte)?;
        Ok(f64::from_le_bytes(byte))
    }
}

impl FromBytes for String {
    #[inline]
    fn from_bytes<T: Seek + Read>(r: &mut T) -> io::Result<Self> {
        let length = r.read_usize_varint()?;
        if length > 1_000_000 {
            return Err(io::Error::new(
                io::ErrorKind::OutOfMemory,
                "attempting to allocate large memory!",
            ));
        }
        let mut buf = vec![0u8; length];
        r.read_exact(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf).to_string())
    }
}

impl<T> FromBytes for Vec<T>
where
    T: FromBytes,
{
    #[inline]
    fn from_bytes<R: Read + Seek>(r: &mut R) -> std::io::Result<Self> {
        let length = r.read_i32_varint()? as usize;
        let mut out = Vec::with_capacity(length);

        for _ in 0..length {
            out.push(T::from_bytes(r)?);
        }

        Ok(out)
    }
}

impl<K, V> FromBytes for HashMap<K, V>
where
    K: FromBytes + Eq + Hash,
    V: FromBytes,
{
    #[inline]
    fn from_bytes<T: io::Seek + io::Read>(r: &mut T) -> io::Result<Self> {
        let length = r.read_i32_varint()? as usize;
        let mut out = HashMap::with_capacity(length);

        for _ in 0..length {
            out.insert(K::from_bytes(r)?, V::from_bytes(r)?);
        }

        Ok(out)
    }
}

impl<T> FromBytes for Box<T>
where
    T: FromBytes,
{
    #[inline]
    fn from_bytes<R: Read + Seek>(r: &mut R) -> std::io::Result<Self> {
        Ok(Box::new(T::from_bytes(r)?))
    }
}
