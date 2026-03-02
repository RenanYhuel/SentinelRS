use std::fmt;
use std::time::Instant;

pub struct Stopwatch(Instant);

pub fn stopwatch() -> Stopwatch {
    Stopwatch(Instant::now())
}

impl Stopwatch {
    pub fn elapsed_ms(&self) -> u128 {
        self.0.elapsed().as_millis()
    }
}

impl fmt::Display for Stopwatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ms = self.elapsed_ms();
        if ms > 0 {
            write!(f, " ({ms}ms)")
        } else {
            Ok(())
        }
    }
}
