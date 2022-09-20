use serde::{Serialize, Serializer};

/// Serialize one `u32` as ssh_format.
pub(crate) fn serialize_u32(int: u32) -> [u8; 4] {
    int.to_be_bytes()
}

pub(crate) enum MaybeOwned<'a, T> {
    Owned(T),
    Borrowed(&'a T),
}

impl<T> MaybeOwned<'_, T> {
    pub(crate) fn as_ref(&self) -> &T {
        use MaybeOwned::*;

        match self {
            Owned(val) => val,
            Borrowed(reference) => reference,
        }
    }
}

impl<T: Serialize> Serialize for MaybeOwned<'_, T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_ref().serialize(serializer)
    }
}
