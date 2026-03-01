use std::sync::atomic::{AtomicU64, Ordering};

static GLOBAL_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn next() -> u64 {
    GLOBAL_SEQ.fetch_add(1, Ordering::SeqCst)
}

pub fn current() -> u64 {
    GLOBAL_SEQ.load(Ordering::SeqCst)
}

pub fn reset(value: u64) {
    GLOBAL_SEQ.store(value, Ordering::SeqCst);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monotonic_increment() {
        reset(0);
        assert_eq!(next(), 0);
        assert_eq!(next(), 1);
        assert_eq!(next(), 2);
        assert_eq!(current(), 3);
    }
}
