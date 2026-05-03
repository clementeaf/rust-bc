;; Minimal key-value store smart contract for Cerulean Ledger DLT demo.
;;
;; Exports two functions:
;;   - "set": writes key "demo" = "hello" to world state
;;   - "get": reads key "demo" from world state (result in memory at offset 64)
;;
;; Host imports (provided by WasmExecutor):
;;   - put_state(key_ptr, key_len, val_ptr, val_len) -> i32
;;   - get_state(key_ptr, key_len, buf_ptr, buf_len) -> i32
;;
;; Return convention: i64 where high 32 bits = ptr, low 32 bits = len of result.
;; Return 0 (ptr=0, len=0) for void results.

(module
  (import "env" "put_state" (func $put_state (param i32 i32 i32 i32) (result i32)))
  (import "env" "get_state" (func $get_state (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)

  ;; Static data: key = "demo" at offset 0, value = "hello" at offset 16
  (data (i32.const 0) "demo")
  (data (i32.const 16) "hello")

  ;; set(): put_state("demo", "hello") -> returns 0 (void)
  (func (export "set") (result i64)
    (drop (call $put_state
      (i32.const 0)   ;; key_ptr
      (i32.const 4)   ;; key_len ("demo")
      (i32.const 16)  ;; val_ptr
      (i32.const 5)   ;; val_len ("hello")
    ))
    (i64.const 0)
  )

  ;; get(): get_state("demo") -> result bytes at offset 64, return ptr|len
  (func (export "get") (result i64)
    (local $read_len i32)
    (local.set $read_len
      (call $get_state
        (i32.const 0)   ;; key_ptr
        (i32.const 4)   ;; key_len ("demo")
        (i32.const 64)  ;; buf_ptr (output)
        (i32.const 32)  ;; buf_len (max)
      )
    )
    ;; Return packed ptr|len: (64 << 32) | read_len
    (i64.or
      (i64.shl (i64.const 64) (i64.const 32))
      (i64.extend_i32_u (local.get $read_len))
    )
  )
)
