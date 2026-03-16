(module
    (import "env" "storage_set" (func $storage_set (param i32 i32 i32 i32)))
    (import "env" "storage_get" (func $storage_get (param i32 i32 i32 i32) (result i32)))
    (import "env" "emit_event" (func $emit_event (param i32 i32 i32 i32)))
    (memory (export "memory") 1)
    (data (i32.const 0) "counter")
    (data (i32.const 7) "\00\00\00\00")
    (data (i32.const 16) "initialized")
    (data (i32.const 32) "\01")

    (func (export "init")
        (call $storage_set
            (i32.const 0) (i32.const 7)
            (i32.const 7) (i32.const 4))
        (call $emit_event
            (i32.const 16) (i32.const 11)
            (i32.const 32) (i32.const 1))
    )

    (func (export "increment")
        (local $current i32)
        (local.set $current
            (call $storage_get
                (i32.const 0) (i32.const 7)
                (i32.const 64) (i32.const 4)))
        (if (i32.eq (local.get $current) (i32.const 4))
            (then
                (i32.store (i32.const 80)
                    (i32.add (i32.load (i32.const 64)) (i32.const 1)))
                (call $storage_set
                    (i32.const 0) (i32.const 7)
                    (i32.const 80) (i32.const 4))
            )
        )
    )

    (func (export "get_counter") (result i32)
        (local $len i32)
        (local.set $len
            (call $storage_get
                (i32.const 0) (i32.const 7)
                (i32.const 96) (i32.const 4)))
        (if (result i32) (i32.eq (local.get $len) (i32.const 4))
            (then (i32.load (i32.const 96)))
            (else (i32.const -1))
        )
    )
)
