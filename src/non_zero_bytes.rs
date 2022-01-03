use std::borrow::{Borrow, ToOwned};
use std::mem::transmute;
use std::num::NonZeroU8;
use std::ops::Deref;

#[derive(Debug, Eq, PartialEq)]
#[repr(transparent)]
pub struct NonZeroByteSlice([u8]);

impl NonZeroByteSlice {
    pub fn new(bytes: &[u8]) -> Option<&Self> {
        for byte in bytes {
            if *byte == 0 {
                return None;
            }
        }

        Some(unsafe { Self::new_unchecked(bytes) })
    }

    /// # Safety
    ///
    /// * `bytes` - Must not contain `0`.
    pub const unsafe fn new_unchecked(bytes: &[u8]) -> &Self {
        transmute(bytes)
    }

    pub const fn into_inner(&self) -> &[u8] {
        &self.0
    }
}

impl ToOwned for NonZeroByteSlice {
    type Owned = NonZeroByteVec;

    fn to_owned(&self) -> Self::Owned {
        self.into()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
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

impl Deref for NonZeroByteVec {
    type Target = NonZeroByteSlice;

    fn deref(&self) -> &Self::Target {
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
        let mut vec = Vec::with_capacity(10);
        for _ in 0..9 {
            vec.push(1);
        }
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
}
