use std::{
    borrow::{Borrow, ToOwned},
    convert::TryFrom,
    error::Error,
    ffi::{CStr, CString},
    fmt,
    num::NonZeroU8,
    ops::Deref,
};

use serde::Serialize;

#[derive(Debug, Eq, PartialEq, Hash, Serialize)]
#[repr(transparent)]
pub struct NonZeroByteSlice([u8]);

impl NonZeroByteSlice {
    pub fn new(bytes: &[u8]) -> Option<&Self> {
        for byte in bytes {
            if *byte == 0 {
                return None;
            }
        }

        // safety: bytes does not contain 0
        Some(unsafe { Self::new_unchecked(bytes) })
    }

    /// # Safety
    ///
    /// * `bytes` - Must not contain `0`.
    pub const unsafe fn new_unchecked(bytes: &[u8]) -> &Self {
        &*(bytes as *const [u8] as *const Self)
    }

    pub const fn into_inner(&self) -> &[u8] {
        &self.0
    }
}

/// The string contains null byte.
#[derive(Debug)]
pub struct NullByteError;

impl fmt::Display for NullByteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NullByteError")
    }
}

impl Error for NullByteError {}

impl<'a> TryFrom<&'a str> for &'a NonZeroByteSlice {
    type Error = NullByteError;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        NonZeroByteSlice::new(s.as_bytes()).ok_or(NullByteError)
    }
}

impl<'a> From<&'a CStr> for &'a NonZeroByteSlice {
    fn from(s: &'a CStr) -> Self {
        // safety: CStr cannot contain 0 byte
        unsafe { NonZeroByteSlice::new_unchecked(s.to_bytes()) }
    }
}

impl ToOwned for NonZeroByteSlice {
    type Owned = NonZeroByteVec;

    fn to_owned(&self) -> Self::Owned {
        self.into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize)]
#[repr(transparent)]
pub struct NonZeroByteVec(Vec<u8>);

impl NonZeroByteVec {
    pub fn new(bytes: Vec<u8>) -> Option<Self> {
        for byte in bytes.iter() {
            if *byte == 0 {
                return None;
            }
        }

        Some(Self(bytes))
    }

    pub fn from_bytes_remove_nul(mut bytes: Vec<u8>) -> Self {
        bytes.retain(|byte| *byte != b'\0');
        Self(bytes)
    }

    /// # Safety
    ///
    /// * `bytes` - Must not contain `0`.
    pub const unsafe fn new_unchecked(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn from_slice(slice: &NonZeroByteSlice) -> Self {
        Self(slice.into_inner().into())
    }

    pub fn push(&mut self, byte: NonZeroU8) {
        self.0.push(byte.get())
    }
}

impl From<&NonZeroByteSlice> for NonZeroByteVec {
    fn from(slice: &NonZeroByteSlice) -> Self {
        Self::from_slice(slice)
    }
}

impl TryFrom<String> for NonZeroByteVec {
    type Error = NullByteError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(s.into_bytes()).ok_or(NullByteError)
    }
}

impl From<CString> for NonZeroByteVec {
    fn from(s: CString) -> Self {
        // safety: CString cannot contain 0 byte
        unsafe { Self::new_unchecked(s.into_bytes()) }
    }
}

impl Deref for NonZeroByteVec {
    type Target = NonZeroByteSlice;

    fn deref(&self) -> &Self::Target {
        // safety: self.0 does not contain 0
        unsafe { NonZeroByteSlice::new_unchecked(&self.0) }
    }
}

impl Borrow<NonZeroByteSlice> for NonZeroByteVec {
    fn borrow(&self) -> &NonZeroByteSlice {
        self.deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_slice_with_zero() {
        let mut vec: Vec<_> = (0..9).collect();
        vec.push(0);

        let option = NonZeroByteSlice::new(&vec);
        debug_assert!(option.is_none(), "{:#?}", option);
    }

    #[test]
    fn test_byte_slice_without_zero() {
        let vec: Vec<_> = (1..102).collect();
        NonZeroByteSlice::new(&vec).unwrap();
    }

    #[test]
    fn test_byte_vec_without_zero() {
        let vec: Vec<_> = (1..102).collect();
        NonZeroByteVec::new(vec).unwrap();
    }

    #[test]
    fn test_byte_vec_from_bytes_remove_nul_zero() {
        let mut vec: Vec<_> = (0..3).collect();
        vec.push(0);
        assert_eq!(NonZeroByteVec::from_bytes_remove_nul(vec).0, vec![1, 2]);
    }
}
