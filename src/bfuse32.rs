//! Implements BinaryFuse16 filters.

use crate::{bfuse_contains_impl, bfuse_from_impl, Filter};
use alloc::{boxed::Box, vec::Vec};
use core::convert::TryFrom;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "bincode")]
use bincode::{Decode, Encode};

/// A `BinaryFuse32` filter is an Xor-like filter with 32-bit fingerprints arranged in a binary-partitioned [fuse graph].
/// `BinaryFuse32`s are similar to [`Fuse32`]s, but their construction is faster, uses less
/// memory, and is more likely to succeed.
///
/// A `BinaryFuse32` filter uses ≈36 bits per entry of the set is it constructed from, and has a false
/// positive rate of effectively zero (1/2^32 =~ 1/4 billion). As with other
/// probabilistic filters, a higher number of entries decreases the bits per
/// entry but increases the false positive rate.
///
/// A `BinaryFuse32` is constructed from a set of 64-bit unsigned integers and is immutable.
/// Construction may fail, but usually only if there are duplicate keys.
///
/// ```
/// # extern crate alloc;
/// use xorf::{Filter, BinaryFuse32};
/// use core::convert::TryFrom;
/// # use alloc::vec::Vec;
/// # use rand::Rng;
///
/// # let mut rng = rand::thread_rng();
/// const SAMPLE_SIZE: usize = 1_000_000;
/// let keys: Vec<u64> = (0..SAMPLE_SIZE).map(|_| rng.gen::<u64>()).collect();
/// let filter = BinaryFuse32::try_from(&keys).unwrap();
///
/// // no false negatives
/// for key in keys {
///     assert!(filter.contains(&key));
/// }
///
/// // bits per entry
/// let bpe = (filter.len() as f64) * 32.0 / (SAMPLE_SIZE as f64);
/// assert!(bpe < 36.2, "Bits per entry is {}", bpe);
///
/// // false positive rate
/// let false_positives: usize = (0..SAMPLE_SIZE)
///     .map(|_| rng.gen::<u64>())
///     .filter(|n| filter.contains(n))
///     .count();
/// let fp_rate: f64 = (false_positives * 100) as f64 / SAMPLE_SIZE as f64;
/// assert!(fp_rate < 0.0000000000000001, "False positive rate is {}", fp_rate);
/// ```
///
/// Serializing and deserializing `BinaryFuse32` filters can be enabled with the [`serde`] feature (or [`bincode`] for bincode).
///
/// [fuse graph]: https://arxiv.org/abs/1907.04749
/// [`Fuse32`]: crate::Fuse32
/// [`serde`]: http://serde.rs
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "bincode", derive(Encode, Decode))]
#[derive(Debug, Clone)]
pub struct BinaryFuse32 {
    seed: u64,
    segment_length: u32,
    segment_length_mask: u32,
    segment_count_length: u32,
    /// The fingerprints for the filter
    pub fingerprints: Box<[u32]>,
}

impl Filter<u64> for BinaryFuse32 {
    /// Returns `true` if the filter contains the specified key.
    /// Has a false positive rate of <0.4%.
    /// Has no false negatives.
    fn contains<Q: crate::Borrow<u64>>(&self, key: &Q) -> bool {
        bfuse_contains_impl!(*key.borrow(), self, fingerprint u32)
    }

    fn len(&self) -> usize {
        self.fingerprints.len()
    }
}

impl BinaryFuse32 {
    /// Try to construct the filter from a key iterator. Can be used directly
    /// if you don't have a contiguous array of u64 keys.
    ///
    /// Note: the iterator will be iterated over multiple times while building
    /// the filter. If using a hash function to map the key, it may be cheaper
    /// just to create a scratch array of hashed keys that you pass in.
    pub fn try_from_iterator<T>(keys: T) -> Result<Self, &'static str>
    where
        T: ExactSizeIterator<Item = u64> + Clone,
    {
        bfuse_from_impl!(keys fingerprint u32, max iter 1_000)
    }
}

impl TryFrom<&[u64]> for BinaryFuse32 {
    type Error = &'static str;

    fn try_from(keys: &[u64]) -> Result<Self, Self::Error> {
        Self::try_from_iterator(keys.iter().copied())
    }
}

impl TryFrom<&Vec<u64>> for BinaryFuse32 {
    type Error = &'static str;

    fn try_from(v: &Vec<u64>) -> Result<Self, Self::Error> {
        Self::try_from_iterator(v.iter().copied())
    }
}

impl TryFrom<Vec<u64>> for BinaryFuse32 {
    type Error = &'static str;

    fn try_from(v: Vec<u64>) -> Result<Self, Self::Error> {
        Self::try_from_iterator(v.iter().copied())
    }
}

#[cfg(test)]
mod test {
    use crate::{BinaryFuse32, Filter};
    use core::convert::TryFrom;

    use alloc::vec::Vec;
    use rand::Rng;

    #[test]
    fn test_initialization() {
        const SAMPLE_SIZE: usize = 1_000_000;
        let mut rng = rand::thread_rng();
        let keys: Vec<u64> = (0..SAMPLE_SIZE).map(|_| rng.gen::<u64>()).collect();

        let filter = BinaryFuse32::try_from(&keys).unwrap();

        for key in keys {
            assert!(filter.contains(&key));
        }
    }

    #[test]
    fn test_bits_per_entry() {
        const SAMPLE_SIZE: usize = 1_000_000;
        let mut rng = rand::thread_rng();
        let keys: Vec<u64> = (0..SAMPLE_SIZE).map(|_| rng.gen::<u64>()).collect();

        let filter = BinaryFuse32::try_from(&keys).unwrap();
        let bpe = (filter.len() as f64) * 32.0 / (SAMPLE_SIZE as f64);

        assert!(bpe < 36.2, "Bits per entry is {}", bpe);
    }

    #[test]
    fn test_false_positives() {
        const SAMPLE_SIZE: usize = 1_000_000;
        let mut rng = rand::thread_rng();
        let keys: Vec<u64> = (0..SAMPLE_SIZE).map(|_| rng.gen::<u64>()).collect();

        let filter = BinaryFuse32::try_from(&keys).unwrap();

        let false_positives: usize = (0..SAMPLE_SIZE)
            .map(|_| rng.gen::<u64>())
            .filter(|n| filter.contains(n))
            .count();
        let fp_rate: f64 = (false_positives * 100) as f64 / SAMPLE_SIZE as f64;
        assert!(
            fp_rate < 0.0000000000000001,
            "False positive rate is {}",
            fp_rate
        );
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(
        expected = "Binary Fuse filters must be constructed from a collection containing all distinct keys."
    )]
    fn test_debug_assert_duplicates() {
        let _ = BinaryFuse32::try_from(vec![1, 2, 1]);
    }
}
