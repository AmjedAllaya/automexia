//! In-memory byte limits shared by application connection stores.
//!
//! Schemas, validation, error mapping, file permissions, replacement and recovery
//! remain with each store. This writer has no filesystem or durability authority.

use std::io::{self, Write};

pub(super) struct BoundedWriter {
    bytes: Vec<u8>,
    maximum: usize,
    limit_message: &'static str,
}

impl BoundedWriter {
    pub(super) fn new(
        maximum: usize,
        initial_capacity: usize,
        limit_message: &'static str,
    ) -> io::Result<Self> {
        if maximum > isize::MAX as usize {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, limit_message));
        }
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(initial_capacity.min(maximum))
            .map_err(|_| allocation_error())?;
        Ok(Self {
            bytes,
            maximum,
            limit_message,
        })
    }

    pub(super) fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl Write for BoundedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let required = self
            .bytes
            .len()
            .checked_add(buffer.len())
            .filter(|required| *required <= self.maximum)
            .ok_or_else(|| io::Error::other(self.limit_message))?;
        if required > self.bytes.capacity() {
            // Retain amortized growth without Vec's implicit doubling past the
            // document ceiling. The allocator's own bookkeeping is not counted.
            let capacity = self
                .bytes
                .capacity()
                .saturating_mul(2)
                .max(required)
                .min(self.maximum);
            self.bytes
                .try_reserve_exact(capacity - self.bytes.len())
                .map_err(|_| allocation_error())?;
        }
        self.bytes.extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // Accepted bytes already reside in the final memory sink, not a file.
        Ok(())
    }
}

fn allocation_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::OutOfMemory,
        "bounded serialization allocation failed",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn writer(maximum: usize, initial: usize) -> BoundedWriter {
        BoundedWriter::new(maximum, initial, "fixture byte limit").unwrap()
    }

    #[test]
    fn zero_exact_and_over_limit_writes_are_atomic_and_byte_based() {
        let mut empty = writer(0, 64);
        assert_eq!(empty.bytes.capacity(), 0);
        assert_eq!(empty.write(&[]).unwrap(), 0);
        assert!(empty.write(b"x").is_err());
        empty.flush().unwrap();
        assert!(empty.into_bytes().is_empty());

        let mut output = writer(5, 2);
        output.write_all("é".as_bytes()).unwrap();
        assert!(output.write(b"1234").is_err());
        assert_eq!(output.bytes, "é".as_bytes());
        output.write_all("界".as_bytes()).unwrap();
        assert_eq!(output.bytes, "é界".as_bytes());
        assert_eq!(output.write(&[]).unwrap(), 0);
        let before = output.bytes.clone();
        let error = output.write(b"untrusted input canary").unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(error.to_string(), "fixture byte limit");
        output.flush().unwrap();
        assert_eq!(output.into_bytes(), before);
    }

    #[test]
    fn capacity_growth_is_capped_for_large_then_tiny_writes() {
        for maximum in [1, 3, 31, 64, 129, 4096] {
            let mut output = writer(maximum, 2);
            output.write_all(&vec![b'x'; maximum / 2 + 1]).unwrap();
            while output.bytes.len() < maximum {
                output.write_all(b"y").unwrap();
                assert!(output.bytes.capacity() <= maximum);
            }
            assert_eq!(output.bytes.len(), maximum);
            assert!(output.write(b"z").is_err());
            assert!(output.bytes.capacity() <= maximum);
        }
    }

    #[test]
    fn invalid_address_space_limit_fails_without_attempting_a_huge_allocation() {
        let error = match BoundedWriter::new(usize::MAX, usize::MAX, "fixture byte limit")
        {
            Ok(_) => panic!("an unaddressable byte budget was accepted"),
            Err(error) => error,
        };
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(error.to_string(), "fixture byte limit");
        assert_eq!(allocation_error().kind(), io::ErrorKind::OutOfMemory);
    }

    #[test]
    fn vectored_and_formatted_writes_follow_the_standard_write_contract() {
        let mut output = writer(7, 0);
        let count = output
            .write_vectored(&[
                io::IoSlice::new(b""),
                io::IoSlice::new(b"abc"),
                io::IoSlice::new(b"def"),
            ])
            .unwrap();
        // std::io::Write's default accepts the first nonempty slice only.
        assert_eq!(count, 3);
        write!(&mut output, "{}", 1234).unwrap();
        output.flush().unwrap();
        assert_eq!(output.into_bytes(), b"abc1234");
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 128,
            rng_seed: proptest::test_runner::RngSeed::Fixed(0x5345_5249_414c),
            ..ProptestConfig::default()
        })]
        #[test]
        fn arbitrary_write_sequences_preserve_the_accepted_prefix(
            maximum in 0usize..512,
            chunks in prop::collection::vec(prop::collection::vec(any::<u8>(), 0..64), 0..64),
        ) {
            let mut output = writer(maximum, 17);
            let mut expected = Vec::new();
            for chunk in chunks {
                let accepted = expected.len() + chunk.len() <= maximum;
                let actual = output.write(&chunk);
                if accepted {
                    expected.extend_from_slice(&chunk);
                    prop_assert_eq!(actual.unwrap(), chunk.len());
                } else {
                    prop_assert!(actual.is_err());
                }
                prop_assert_eq!(&output.bytes, &expected);
                prop_assert!(output.bytes.capacity() <= maximum);
                output.flush().unwrap();
            }
            prop_assert_eq!(output.into_bytes(), expected);
        }
    }
}
