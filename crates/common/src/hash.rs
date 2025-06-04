pub const fn get_32bit_hash_const(s: &str) -> i32 {
    let mut hash1: i32 = 5381;
    let mut hash2: i32 = hash1;

    let bytes = s.as_bytes();
    let length = bytes.len();

    let mut i = 0;
    while i < length {
        hash1 = ((hash1 << 5).wrapping_add(hash1)) ^ (bytes[i] as i32);

        if i + 1 < length {
            hash2 = ((hash2 << 5).wrapping_add(hash2)) ^ (bytes[i + 1] as i32);
        }

        i += 2;
    }

    hash1.wrapping_add(hash2.wrapping_mul(1566083941))
}

#[inline]
pub fn get_64bit_hash_const(s: &str) -> u64 {
    xxhash_rust::const_xxh64::xxh64(s.as_bytes(), 0)
}
