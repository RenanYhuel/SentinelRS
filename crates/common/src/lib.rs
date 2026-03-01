pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/sentinel.common.rs"));
}

pub mod batch_id;
pub mod canonicalize;
pub mod crypto;
pub mod metric_json;
pub mod nats_config;
pub mod seq;
