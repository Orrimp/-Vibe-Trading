//! Fixed-capacity ring buffer for streaming indicator computations.
//!
//! Pre-allocated at construction so the hot path has no heap allocation.
//! Used by v1 `features::cross_sectional` for per-symbol price histories.

use rust_decimal::Decimal;

/// A fixed-capacity circular buffer of `Decimal` values.
///
/// - `push` inserts a new value, evicting the oldest when full.
/// - `len()` / `is_full()` report current fill level.
/// - `last()` returns the most-recently-pushed value.
/// - `get_back(n)` returns the value `n` positions behind the most recent
///   (0 == last, 1 == second-to-last, …).
#[derive(Debug, Clone)]
pub struct RingBuffer {
    buf: Vec<Decimal>,
    /// Index of the slot where the *next* push will write.
    head: usize,
    /// Current fill level (0 ..= capacity).
    len: usize,
    capacity: usize,
}

impl RingBuffer {
    /// Create a ring buffer with the given capacity.
    ///
    /// # Panics
    ///
    /// Panics if `capacity == 0`.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "RingBuffer capacity must be > 0");
        Self {
            buf: vec![Decimal::ZERO; capacity],
            head: 0,
            len: 0,
            capacity,
        }
    }

    /// Push a value, evicting the oldest if the buffer is full.
    pub fn push(&mut self, value: Decimal) {
        self.buf[self.head] = value;
        self.head = (self.head + 1) % self.capacity;
        if self.len < self.capacity {
            self.len += 1;
        }
    }

    /// Number of values currently stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if the buffer holds no values.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns `true` when the buffer has been filled to capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.len == self.capacity
    }

    /// The most-recently-pushed value, or `None` if empty.
    #[must_use]
    pub fn last(&self) -> Option<Decimal> {
        if self.len == 0 {
            return None;
        }
        // `head` points to the slot where the *next* push goes,
        // so the last-pushed slot is `(head - 1 + capacity) % capacity`.
        let idx = (self.head + self.capacity - 1) % self.capacity;
        Some(self.buf[idx])
    }

    /// Value `n` positions behind the most recent (0 == last, 1 == second-to-last, …).
    ///
    /// Returns `None` if `n >= self.len()`.
    #[must_use]
    pub fn get_back(&self, n: usize) -> Option<Decimal> {
        if n >= self.len {
            return None;
        }
        let idx = (self.head + self.capacity - 1 - n) % self.capacity;
        Some(self.buf[idx])
    }

    /// Capacity (maximum number of values the buffer can hold).
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn basic_push_and_last() {
        let mut rb = RingBuffer::new(3);
        assert!(rb.last().is_none());
        rb.push(dec!(1));
        assert_eq!(rb.last(), Some(dec!(1)));
        rb.push(dec!(2));
        assert_eq!(rb.last(), Some(dec!(2)));
        rb.push(dec!(3));
        assert_eq!(rb.last(), Some(dec!(3)));
        // Overwrite oldest (1)
        rb.push(dec!(4));
        assert_eq!(rb.last(), Some(dec!(4)));
        assert_eq!(rb.len(), 3); // still full
    }

    #[test]
    fn get_back_ordering() {
        let mut rb = RingBuffer::new(5);
        for i in 1u32..=5 {
            rb.push(Decimal::from(i));
        }
        // After pushing [1,2,3,4,5], get_back(0)=5, get_back(1)=4, ...
        assert_eq!(rb.get_back(0), Some(dec!(5)));
        assert_eq!(rb.get_back(1), Some(dec!(4)));
        assert_eq!(rb.get_back(2), Some(dec!(3)));
        assert_eq!(rb.get_back(3), Some(dec!(2)));
        assert_eq!(rb.get_back(4), Some(dec!(1)));
        assert!(rb.get_back(5).is_none());
    }

    #[test]
    fn is_full_flag() {
        let mut rb = RingBuffer::new(3);
        assert!(!rb.is_full());
        rb.push(dec!(1));
        rb.push(dec!(2));
        assert!(!rb.is_full());
        rb.push(dec!(3));
        assert!(rb.is_full());
        rb.push(dec!(4)); // overwrite — still full
        assert!(rb.is_full());
    }

    #[test]
    fn wrap_around_get_back() {
        let mut rb = RingBuffer::new(4);
        // Push 6 items — oldest 2 are evicted
        for i in 1u32..=6 {
            rb.push(Decimal::from(i));
        }
        // Stored: [3, 4, 5, 6]
        assert_eq!(rb.get_back(0), Some(dec!(6)));
        assert_eq!(rb.get_back(1), Some(dec!(5)));
        assert_eq!(rb.get_back(2), Some(dec!(4)));
        assert_eq!(rb.get_back(3), Some(dec!(3)));
        assert!(rb.get_back(4).is_none());
    }
}
