// Pure accounting stays independently testable without installing an allocator.
#[derive(Clone, Copy, Debug)]
pub struct Accounting {
    pub live: usize,
    pub peak: usize,
    pub valid: bool,
}
impl Accounting {
    pub const fn new() -> Self {
        Self {
            live: 0,
            peak: 0,
            valid: true,
        }
    }
    pub fn allocate(&mut self, size: usize) {
        if let Some(next) = self.live.checked_add(size) {
            self.live = next;
            self.peak = self.peak.max(next);
        } else {
            self.valid = false;
        }
    }
    pub fn release(&mut self, size: usize) {
        if let Some(next) = self.live.checked_sub(size) {
            self.live = next;
        } else {
            self.valid = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Accounting;

    #[test]
    fn balanced_lifetimes_record_exact_peak_and_cleanup() {
        let mut count = Accounting::new();
        count.allocate(7);
        count.allocate(13);
        count.release(7);
        count.allocate(4);
        count.release(17);
        assert!(count.valid);
        assert_eq!(count.live, 0);
        assert_eq!(count.peak, 20);
    }

    #[test]
    fn arithmetic_errors_are_reported_without_unwinding() {
        // An allocator callback must never unwind, even when the test fails.
        let mut underflow = Accounting::new();
        underflow.release(1);
        assert!(!underflow.valid);
        assert_eq!(underflow.live, 0);
        let mut overflow = Accounting {
            live: usize::MAX,
            peak: usize::MAX,
            valid: true,
        };
        overflow.allocate(1);
        assert!(!overflow.valid);
        assert_eq!(overflow.live, usize::MAX);
    }
}
