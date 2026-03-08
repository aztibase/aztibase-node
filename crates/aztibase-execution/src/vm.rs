use std::collections::HashMap;

use anyhow::Result;

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

#[derive(Clone, Debug)]
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

fn wt_err(msg: &str) -> wasmtime::Error {
    wasmtime::Error::msg(msg.to_string())
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

        let engine = wasmtime::Engine::new(&wasm_config)
            .map_err(|e| anyhow::anyhow!("failed to create wasmtime engine: {e}"))?;

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
            .map_err(|e| anyhow::anyhow!("failed to compile WASM module: {e}"))?;

        let mut store = wasmtime::Store::new(
            &self.engine,
            HostState {
                storage: HashMap::new(),
                events: Vec::new(),
            },
        );
        store
            .set_fuel(self.fuel_limit)
            .map_err(|e| anyhow::anyhow!("failed to set fuel: {e}"))?;

        let mut linker = wasmtime::Linker::new(&self.engine);
        Self::register_host_functions(&mut linker)?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| anyhow::anyhow!("failed to instantiate module: {e}"))?;

        let func = instance
            .get_func(&mut store, func_name)
            .ok_or_else(|| anyhow::anyhow!("function not found: {func_name}"))?;

        let num_results = func.ty(&store).results().len();
        let mut results = vec![wasmtime::Val::I32(0); num_results];

        func.call(&mut store, args, &mut results)
            .map_err(|e| anyhow::anyhow!("execution failed: {e}"))?;

        let fuel_remaining = store
            .get_fuel()
            .map_err(|e| anyhow::anyhow!("failed to get fuel: {e}"))?;
        let fuel_consumed = self.fuel_limit - fuel_remaining;

        let host_state = store.into_data();

        Ok(ExecutionResult {
            values: results,
            fuel_consumed,
            storage: host_state.storage,
            events: host_state.events,
        })
    }

    /// Execute a WASM module with pre-loaded storage. Used for contract calls
    /// where existing contract state needs to be available to host functions.
    pub fn execute_with_storage(
        &self,
        wasm_bytes: &[u8],
        func_name: &str,
        args: &[wasmtime::Val],
        initial_storage: std::collections::BTreeMap<Vec<u8>, Vec<u8>>,
    ) -> Result<ExecutionResult> {
        let module = wasmtime::Module::new(&self.engine, wasm_bytes)
            .map_err(|e| anyhow::anyhow!("failed to compile WASM module: {e}"))?;

        let storage: HashMap<Vec<u8>, Vec<u8>> = initial_storage.into_iter().collect();
        let mut store = wasmtime::Store::new(
            &self.engine,
            HostState {
                storage,
                events: Vec::new(),
            },
        );
        store
            .set_fuel(self.fuel_limit)
            .map_err(|e| anyhow::anyhow!("failed to set fuel: {e}"))?;

        let mut linker = wasmtime::Linker::new(&self.engine);
        Self::register_host_functions(&mut linker)?;

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| anyhow::anyhow!("failed to instantiate module: {e}"))?;

        let func = instance
            .get_func(&mut store, func_name)
            .ok_or_else(|| anyhow::anyhow!("function not found: {func_name}"))?;

        let num_results = func.ty(&store).results().len();
        let mut results = vec![wasmtime::Val::I32(0); num_results];

        func.call(&mut store, args, &mut results)
            .map_err(|e| anyhow::anyhow!("execution failed: {e}"))?;

        let fuel_remaining = store
            .get_fuel()
            .map_err(|e| anyhow::anyhow!("failed to get fuel: {e}"))?;
        let fuel_consumed = self.fuel_limit - fuel_remaining;

        let host_state = store.into_data();

        Ok(ExecutionResult {
            values: results,
            fuel_consumed,
            storage: host_state.storage,
            events: host_state.events,
        })
    }

    fn read_guest_memory(data: &[u8], ptr: i32, len: i32) -> Result<Vec<u8>, wasmtime::Error> {
        if ptr < 0 || len < 0 {
            return Err(wt_err("negative pointer or length"));
        }
        let start = ptr as usize;
        let end = start
            .checked_add(len as usize)
            .ok_or_else(|| wt_err("pointer arithmetic overflow"))?;
        if end > data.len() {
            return Err(wasmtime::Error::msg(format!(
                "out-of-bounds memory access: {end} > {}",
                data.len()
            )));
        }
        Ok(data[start..end].to_vec())
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
                 -> Result<(), wasmtime::Error> {
                    let memory = caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .ok_or_else(|| wt_err("missing memory export"))?;

                    let data = memory.data(&caller);
                    let key = Self::read_guest_memory(data, key_ptr, key_len)?;
                    let val = Self::read_guest_memory(data, val_ptr, val_len)?;

                    caller.data_mut().storage.insert(key, val);
                    Ok(())
                },
            )
            .map_err(|e| anyhow::anyhow!("failed to register storage_set: {e}"))?;

        linker
            .func_wrap(
                "env",
                "storage_get",
                |mut caller: wasmtime::Caller<'_, HostState>,
                 key_ptr: i32,
                 key_len: i32,
                 out_ptr: i32,
                 out_cap: i32|
                 -> Result<i32, wasmtime::Error> {
                    let memory = caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .ok_or_else(|| wt_err("missing memory export"))?;

                    let data = memory.data(&caller);
                    let key = Self::read_guest_memory(data, key_ptr, key_len)?;

                    match caller.data().storage.get(&key) {
                        Some(val) => {
                            if out_ptr < 0 || out_cap < 0 {
                                return Err(wt_err("negative output pointer or capacity"));
                            }
                            let write_len = val.len().min(out_cap as usize);
                            let out_start = out_ptr as usize;
                            let out_end = out_start
                                .checked_add(write_len)
                                .ok_or_else(|| wt_err("output pointer overflow"))?;

                            let val_copy = val[..write_len].to_vec();

                            let memory = caller
                                .get_export("memory")
                                .and_then(|e| e.into_memory())
                                .ok_or_else(|| wt_err("missing memory export"))?;

                            let mem_data = memory.data_mut(&mut caller);
                            if out_end > mem_data.len() {
                                return Err(wasmtime::Error::msg(format!(
                                    "out-of-bounds write: {out_end} > {}",
                                    mem_data.len()
                                )));
                            }
                            mem_data[out_start..out_end].copy_from_slice(&val_copy);

                            Ok(write_len as i32)
                        }
                        None => Ok(-1),
                    }
                },
            )
            .map_err(|e| anyhow::anyhow!("failed to register storage_get: {e}"))?;

        linker
            .func_wrap(
                "env",
                "emit_event",
                |mut caller: wasmtime::Caller<'_, HostState>,
                 topic_ptr: i32,
                 topic_len: i32,
                 data_ptr: i32,
                 data_len: i32|
                 -> Result<(), wasmtime::Error> {
                    let memory = caller
                        .get_export("memory")
                        .and_then(|e| e.into_memory())
                        .ok_or_else(|| wt_err("missing memory export"))?;

                    let mem = memory.data(&caller);
                    let topic = Self::read_guest_memory(mem, topic_ptr, topic_len)?;
                    let data = Self::read_guest_memory(mem, data_ptr, data_len)?;

                    caller.data_mut().events.push(ContractEvent { topic, data });
                    Ok(())
                },
            )
            .map_err(|e| anyhow::anyhow!("failed to register emit_event: {e}"))?;

        Ok(())
    }
}
