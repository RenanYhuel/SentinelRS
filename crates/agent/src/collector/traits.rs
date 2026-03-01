use sentinel_common::proto::Metric;

pub trait Collector: Send + Sync {
    fn collect(&self) -> Vec<Metric>;
}
