pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/sentinel.common.rs"));
}

pub mod canonicalize;
pub mod batch_id;
pub mod seq;
pub mod metric_json;
