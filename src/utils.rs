/// Serialize one `u32` as ssh_format.
pub(crate) fn serialize_u32(int: u32) -> [u8; 4] {
    int.to_be_bytes()
}
