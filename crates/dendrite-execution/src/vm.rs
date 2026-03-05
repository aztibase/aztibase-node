use std::collections::HashMap;

use anyhow::{Context, Result};

pub struct EngineConfig {
    pub fuel_limit: u64,
    pub enable_simd: bool,
    pub enable_threads: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            fuel_limit: 100_000_000,
            enable_simd: false,
            enable_threads: false,
        }
    }
}

pub struct ContractEvent {
    pub topic: Vec<u8>,
    pub data: Vec<u8>,
}

pub struct HostState {
    pub storage: HashMap<Vec<u8>, Vec<u8>>,
    pub events: Vec<ContractEvent>,
}

pub struct ExecutionResult {
    pub values: Vec<wasmtime::Val>,
    pub fuel_consumed: u64,
    pub storage: HashMap<Vec<u8>, Vec<u8>>,
    pub events: Vec<ContractEvent>,
}

pub struct ExecutionEngine {
    engine: wasmtime::Engine,
    fuel_limit: u64,
}

impl ExecutionEngine {
    pub fn new(config: EngineConfig) -> Result<Self> {
        let mut wasm_config = wasmtime::Config::new();
        wasm_config.consume_fuel(true);
        wasm_config.wasm_simd(config.enable_simd);
        wasm_config.wasm_relaxed_simd(config.enable_simd);
        wasm_config.wasm_threads(config.enable_threads);

        let engine =
            wasmtime::Engine::new(&wasm_config).context("Failed to create wasmtime engine")?;

        Ok(Self {
            engine,
            fuel_limit: config.fuel_limit,
        })
    }

    pub fn execute(
        &self,
        wasm_bytes: &[u8],
        func_name: &str,
        args: &[wasmtime::Val],
    ) -> Result<ExecutionResult> {
        let module = wasmtime::Module::new(&self.engine, wasm_bytes)
            .context("Failed to compile WASM module")?;

        let mut store = wasmtime::Store::new(
            &self.engine,
            HostState {
                storage: HashMap::new(),
                events: Vec::new(),
            },
        );
        store
            .set_fuel(self.fuel_limit)
            .context("Failed to set fuel")?;

        let mut linker = wasmtime::Linker::new(&self.engine);
        Self::register_host_functions(&mut linker)?;

        let instance = linker
            .instantiate(&mut store, &module)
            .context("Failed to instantiate module")?;

        let func = instance
            .get_func(&mut store, func_name)
            .context("Function not found")?;

        let num_results = func.ty(&store).results().len();
        let mut results = vec![wasmtime::Val::I32(0); num_results];

        func.call(&mut store, args, &mut results)
            .context("Execution failed")?;

        let fuel_remaining = store.get_fuel().context("Failed to get fuel")?;
        let fuel_consumed = self.fuel_limit - fuel_remaining;

        let host_state = store.into_data();

        Ok(ExecutionResult {
            values: results,
            fuel_consumed,
            storage: host_state.storage,
            events: host_state.events,
        })
    }

    fn register_host_functions(linker: &mut wasmtime::Linker<HostState>) -> Result<()> {
        linker
            .func_wrap(
                "env",
                "storage_set",
                |mut caller: wasmtime::Caller<'_, HostState>,
                 key_ptr: i32,
                 key_len: i32,
                 val_ptr: i32,
                 val_len: i32|
                 -> anyhow::Result<()> {
                    let memory = caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .ok_or_else(|| anyhow::anyhow!("missing memory export"))?;

                    let data = memory.data(&caller);
                    let key = data[key_ptr as usize..(key_ptr + key_len) as usize].to_vec();
                    let val = data[val_ptr as usize..(val_ptr + val_len) as usize].to_vec();

                    caller.data_mut().storage.insert(key, val);
                    Ok(())
                },
            )
            .context("Failed to register storage_set")?;

        linker
            .func_wrap(
                "env",
                "storage_get",
                |mut caller: wasmtime::Caller<'_, HostState>,
                 key_ptr: i32,
                 key_len: i32,
                 out_ptr: i32,
                 out_cap: i32|
                 -> anyhow::Result<i32> {
                    let memory = caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .ok_or_else(|| anyhow::anyhow!("missing memory export"))?;

                    let data = memory.data(&caller);
                    let key = data[key_ptr as usize..(key_ptr + key_len) as usize].to_vec();

                    match caller.data().storage.get(&key) {
                        Some(val) => {
                            let write_len = val.len().min(out_cap as usize);
                            let val_copy = val[..write_len].to_vec();

                            let memory = caller
                                .get_export("memory")
                                .and_then(|e| e.into_memory())
                                .ok_or_else(|| anyhow::anyhow!("missing memory export"))?;

                            memory.data_mut(&mut caller)
                                [out_ptr as usize..out_ptr as usize + write_len]
                                .copy_from_slice(&val_copy);

                            Ok(write_len as i32)
                        }
                        None => Ok(-1),
                    }
                },
            )
            .context("Failed to register storage_get")?;

        linker
            .func_wrap(
                "env",
                "emit_event",
                |mut caller: wasmtime::Caller<'_, HostState>,
                 topic_ptr: i32,
                 topic_len: i32,
                 data_ptr: i32,
                 data_len: i32|
                 -> anyhow::Result<()> {
                    let memory = caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .ok_or_else(|| anyhow::anyhow!("missing memory export"))?;

                    let mem = memory.data(&caller);
                    let topic = mem[topic_ptr as usize..(topic_ptr + topic_len) as usize].to_vec();
                    let data = mem[data_ptr as usize..(data_ptr + data_len) as usize].to_vec();

                    caller.data_mut().events.push(ContractEvent { topic, data });
                    Ok(())
                },
            )
            .context("Failed to register emit_event")?;

        Ok(())
    }
}
