;; multi_gauge.wat
;; Emits three gauge metrics in a single collection cycle.

(module
    (import "sentinel" "emit_metric_json"
        (func $emit (param i32 i32) (result i32)))
    (import "sentinel" "log"
        (func $log (param i32 i32)))

    (memory (export "memory") 1)

    ;; Metric 1: connections_active (offset 0, 50 bytes)
    (data (i32.const 0)
        "{\"name\":\"connections_active\",\"value\":42}")
    ;; Metric 2: requests_per_sec (offset 64, 48 bytes)
    (data (i32.const 64)
        "{\"name\":\"requests_per_sec\",\"value\":1500}")
    ;; Metric 3: error_rate (offset 128, 40 bytes)
    (data (i32.const 128)
        "{\"name\":\"error_rate\",\"value\":0.02}")
    ;; Log message (offset 192, 22 bytes)
    (data (i32.const 192)
        "multi_gauge collected!")

    (func (export "collect") (result i32)
        (call $emit (i32.const 0) (i32.const 39))
        drop
        (call $emit (i32.const 64) (i32.const 39))
        drop
        (call $emit (i32.const 128) (i32.const 33))
        drop
        (call $log (i32.const 192) (i32.const 21))
        (i32.const 0)
    )
)
