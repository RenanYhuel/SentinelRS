;; hello_metrics.wat
;; Example SentinelRS WASM plugin that emits a metric and logs a message.
;;
;; Build to .wasm: wat2wasm hello_metrics.wat -o hello_metrics.wasm
;; Or load directly — wasmtime supports WAT natively.
;;
;; Host API:
;;   sentinel::emit_metric_json(ptr: i32, len: i32) -> i32
;;     Emits a JSON metric string. Returns 0 on success, -1 if max_metrics exceeded.
;;   sentinel::log(ptr: i32, len: i32)
;;     Logs a message from the plugin.
;;
;; The entry function must return i32: 0 = success, non-zero = error.

(module
    (import "sentinel" "emit_metric_json"
        (func $emit (param i32 i32) (result i32)))
    (import "sentinel" "log"
        (func $log (param i32 i32)))

    (memory (export "memory") 1)

    ;; Static data in linear memory
    ;; Metric JSON at offset 0 (48 bytes)
    (data (i32.const 0)
        "{\"name\":\"hello\",\"value\":1,\"labels\":{\"src\":\"wasm\"}}")
    ;; Log message at offset 64 (28 bytes)
    (data (i32.const 64)
        "hello_metrics plugin running")

    (func (export "collect") (result i32)
        ;; Log a message
        (call $log (i32.const 64) (i32.const 28))

        ;; Emit a metric
        (call $emit (i32.const 0) (i32.const 48))
        drop

        ;; Return success
        (i32.const 0)
    )
)
