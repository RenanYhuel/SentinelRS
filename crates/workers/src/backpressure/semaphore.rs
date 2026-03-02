use std::sync::Arc;
use tokio::sync::Semaphore;

pub struct BatchSemaphore {
    inner: Arc<Semaphore>,
    max: usize,
}

impl BatchSemaphore {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            inner: Arc::new(Semaphore::new(max_concurrent)),
            max: max_concurrent,
        }
    }

    pub async fn acquire(&self) -> tokio::sync::OwnedSemaphorePermit {
        Arc::clone(&self.inner)
            .acquire_owned()
            .await
            .expect("semaphore closed")
    }

    pub fn available(&self) -> usize {
        self.inner.available_permits()
    }

    pub fn in_use(&self) -> usize {
        self.max - self.inner.available_permits()
    }

    pub fn max(&self) -> usize {
        self.max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tracks_permits() {
        let sem = BatchSemaphore::new(3);
        assert_eq!(sem.available(), 3);
        assert_eq!(sem.in_use(), 0);

        let _p1 = sem.acquire().await;
        assert_eq!(sem.available(), 2);
        assert_eq!(sem.in_use(), 1);

        let _p2 = sem.acquire().await;
        assert_eq!(sem.in_use(), 2);
    }

    #[tokio::test]
    async fn releases_on_drop() {
        let sem = BatchSemaphore::new(1);
        {
            let _p = sem.acquire().await;
            assert_eq!(sem.available(), 0);
        }
        assert_eq!(sem.available(), 1);
    }
}
