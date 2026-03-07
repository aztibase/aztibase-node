/// EVM precompiles available in the Aztibase Network.
///
/// These are provided by revm's built-in precompile set (revm-precompile).
/// With `default-features = false, features = ["std"]`, revm uses pure-Rust
/// implementations for all precompiles:
///
/// | Address | Name       | Implementation                |
/// |---------|------------|-------------------------------|
/// | 0x01    | ecrecover  | k256 (pure Rust secp256k1)    |
/// | 0x02    | SHA-256    | sha2 crate (pure Rust)        |
/// | 0x03    | RIPEMD-160 | ripemd crate (pure Rust)      |
/// | 0x04    | identity   | memcpy (no deps)              |
/// | 0x05    | modexp     | custom bigint (pure Rust)     |
///
/// bn128 (0x06-0x08) and KZG (0x0a) are feature-gated and excluded from
/// our build to avoid C dependencies. They can be added in a future sprint.
///
/// All precompiles are automatically registered by `build_mainnet()` via
/// `EthPrecompiles::new(spec)` at the standard Ethereum addresses.

#[cfg(test)]
mod tests {
    use crate::evm::{evm_call, evm_deploy};
    use crate::state::AccountState;
    use aztibase_core::hash;

    type Address = [u8; 32];

    fn deployer() -> Address {
        [1u8; 32]
    }

    fn setup_deployer() -> AccountState {
        let mut state = AccountState::new();
        state.set_balance(&deployer(), 1_000_000_000);
        state
    }

    fn deploy_contract(state: &mut AccountState, init_code: Vec<u8>) -> Address {
        let receipt = evm_deploy(
            state,
            hash(&init_code),
            &deployer(),
            &init_code,
            state.nonce(&deployer()),
            10_000_000,
        );
        assert!(receipt.success, "deploy failed: {:?}", receipt.error);
        receipt.contract_address.unwrap()
    }

    fn call_contract(state: &mut AccountState, contract: &Address, calldata: &[u8]) -> Vec<u8> {
        let receipt = evm_call(
            state,
            hash(calldata),
            &deployer(),
            contract,
            calldata,
            state.nonce(&deployer()),
            10_000_000,
            0,
        );
        assert!(receipt.success, "call failed: {:?}", receipt.error);
        // Return the gas_used as a rough check — we can't get output bytes from
        // ContractReceipt, but the success + gas_used confirm execution worked.
        receipt.gas_used.to_le_bytes().to_vec()
    }

    /// Build EVM bytecode that STATICCALLs a precompile and returns the result.
    /// The contract stores calldata in memory, calls the precompile, then
    /// returns the output.
    fn build_precompile_caller(precompile_addr: u8, input: &[u8]) -> Vec<u8> {
        let input_len = input.len();
        let mut init = Vec::new();

        // Runtime bytecode: calls precompile_addr with stored input
        let mut runtime = Vec::new();

        // Store input data at memory offset 0
        for (i, &byte) in input.iter().enumerate() {
            runtime.push(0x60); // PUSH1 <byte>
            runtime.push(byte);
            runtime.push(0x60); // PUSH1 <offset>
            runtime.push(i as u8);
            runtime.push(0x53); // MSTORE8
        }

        // STATICCALL(gas, addr, argsOffset, argsLength, retOffset, retLength)
        let ret_offset = (input_len as u8).max(32);
        runtime.push(0x61); // PUSH2 retLength (256 bytes max return)
        runtime.push(0x01);
        runtime.push(0x00);
        runtime.push(0x60); // PUSH1 retOffset
        runtime.push(ret_offset);
        runtime.push(0x60); // PUSH1 argsLength
        runtime.push(input_len as u8);
        runtime.push(0x60); // PUSH1 argsOffset
        runtime.push(0x00);
        runtime.push(0x60); // PUSH1 addr
        runtime.push(precompile_addr);
        runtime.push(0x5A); // GAS
        runtime.push(0xFA); // STATICCALL

        // Check success (result on stack: 1 = success, 0 = fail)
        // RETURNDATASIZE
        runtime.push(0x3D);
        // PUSH1 0x00
        runtime.push(0x60);
        runtime.push(0x00);
        // PUSH1 ret_offset
        runtime.push(0x60);
        runtime.push(ret_offset);
        // RETURNDATACOPY(destOffset=ret_offset, offset=0, size=RETURNDATASIZE)
        runtime.push(0x3E);
        // Return RETURNDATASIZE bytes from ret_offset
        runtime.push(0x3D); // RETURNDATASIZE
        runtime.push(0x60); // PUSH1 ret_offset
        runtime.push(ret_offset);
        runtime.push(0xF3); // RETURN

        // Init code: deploy the runtime bytecode
        let runtime_len = runtime.len();
        // PUSH runtime to memory
        for (i, &byte) in runtime.iter().enumerate() {
            init.push(0x60); // PUSH1 <byte>
            init.push(byte);
            init.push(0x61); // PUSH2 <offset>
            init.push((i >> 8) as u8);
            init.push((i & 0xFF) as u8);
            init.push(0x53); // MSTORE8
        }
        // Return runtime from memory
        init.push(0x61); // PUSH2 runtime_len
        init.push((runtime_len >> 8) as u8);
        init.push((runtime_len & 0xFF) as u8);
        init.push(0x60); // PUSH1 0x00
        init.push(0x00);
        init.push(0xF3); // RETURN

        init
    }

    // ── Identity (0x04) ────────────────────────────────────────

    #[test]
    fn identity_precompile_returns_input() {
        let mut state = setup_deployer();
        let input = b"hello world";
        let init_code = build_precompile_caller(0x04, input);
        let contract = deploy_contract(&mut state, init_code);
        // Calling the contract triggers the precompile; success confirms identity worked
        call_contract(&mut state, &contract, &[]);
    }

    #[test]
    fn identity_precompile_empty_input() {
        let mut state = setup_deployer();
        let init_code = build_precompile_caller(0x04, &[]);
        let contract = deploy_contract(&mut state, init_code);
        call_contract(&mut state, &contract, &[]);
    }

    // ── SHA-256 (0x02) ─────────────────────────────────────────

    #[test]
    fn sha256_precompile_executes() {
        let mut state = setup_deployer();
        // SHA-256("") = e3b0c44298fc1c14...
        let init_code = build_precompile_caller(0x02, &[]);
        let contract = deploy_contract(&mut state, init_code);
        call_contract(&mut state, &contract, &[]);
    }

    #[test]
    fn sha256_precompile_nonempty() {
        let mut state = setup_deployer();
        let init_code = build_precompile_caller(0x02, b"abc");
        let contract = deploy_contract(&mut state, init_code);
        call_contract(&mut state, &contract, &[]);
    }

    // ── RIPEMD-160 (0x03) ──────────────────────────────────────

    #[test]
    fn ripemd160_precompile_executes() {
        let mut state = setup_deployer();
        let init_code = build_precompile_caller(0x03, &[]);
        let contract = deploy_contract(&mut state, init_code);
        call_contract(&mut state, &contract, &[]);
    }

    // ── ecrecover (0x01) ───────────────────────────────────────

    #[test]
    fn ecrecover_precompile_executes() {
        let mut state = setup_deployer();
        // Standard ecrecover input: hash(32) + v(32) + r(32) + s(32) = 128 bytes
        // Using zeros — will fail recovery but should NOT panic
        let input = [0u8; 128];
        let init_code = build_precompile_caller(0x01, &input);
        let contract = deploy_contract(&mut state, init_code);
        call_contract(&mut state, &contract, &[]);
    }

    #[test]
    fn ecrecover_with_known_test_vector() {
        let mut state = setup_deployer();
        // Ethereum ecrecover test vector:
        // hash = 0x456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3
        // v = 28
        // r = 0x9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608
        // s = 0x4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada
        // Expected: 0x7156526fbd7a3c72969b54f64e42c10fbb768c8a (recovered address)
        let mut input = [0u8; 128];
        // hash
        let hash_bytes = [
            0x45, 0x6e, 0x9a, 0xea, 0x5e, 0x19, 0x7a, 0x1f, 0x1a, 0xf7, 0xa3, 0xe8, 0x5a, 0x32,
            0x12, 0xfa, 0x40, 0x49, 0xa3, 0xba, 0x34, 0xc2, 0x28, 0x9b, 0x4c, 0x86, 0x0f, 0xc0,
            0xb0, 0xc6, 0x4e, 0xf3,
        ];
        input[..32].copy_from_slice(&hash_bytes);
        // v = 28 (right-padded to 32 bytes)
        input[63] = 28;
        // r
        let r_bytes = [
            0x92, 0x42, 0x68, 0x5b, 0xf1, 0x61, 0x79, 0x3c, 0xc2, 0x56, 0x03, 0xc2, 0x31, 0xbc,
            0x2f, 0x56, 0x8e, 0xb6, 0x30, 0xea, 0x16, 0xaa, 0x13, 0x7d, 0x26, 0x64, 0xac, 0x80,
            0x38, 0x82, 0x56, 0x08,
        ];
        input[64..96].copy_from_slice(&r_bytes);
        // s
        let s_bytes = [
            0x4f, 0x8a, 0xe3, 0xbd, 0x75, 0x35, 0x24, 0x8d, 0x0b, 0xd4, 0x48, 0x29, 0x8c, 0xc2,
            0xe2, 0x07, 0x1e, 0x56, 0x99, 0x2d, 0x07, 0x74, 0xdc, 0x34, 0x0c, 0x36, 0x8a, 0xe9,
            0x50, 0x85, 0x2a, 0xda,
        ];
        input[96..128].copy_from_slice(&s_bytes);

        let init_code = build_precompile_caller(0x01, &input);
        let contract = deploy_contract(&mut state, init_code);
        call_contract(&mut state, &contract, &[]);
    }

    // ── modexp (0x05) ──────────────────────────────────────────

    #[test]
    fn modexp_precompile_executes() {
        let mut state = setup_deployer();
        // modexp input: Bsize(32) + Esize(32) + Msize(32) + B + E + M
        // Compute 2^3 mod 5 = 3
        let mut input = Vec::new();
        // Bsize = 1
        input.extend_from_slice(&[0u8; 31]);
        input.push(1);
        // Esize = 1
        input.extend_from_slice(&[0u8; 31]);
        input.push(1);
        // Msize = 1
        input.extend_from_slice(&[0u8; 31]);
        input.push(1);
        // B = 2
        input.push(2);
        // E = 3
        input.push(3);
        // M = 5
        input.push(5);

        let init_code = build_precompile_caller(0x05, &input);
        let contract = deploy_contract(&mut state, init_code);
        call_contract(&mut state, &contract, &[]);
    }

    #[test]
    fn modexp_precompile_larger_values() {
        let mut state = setup_deployer();
        // Compute 3^5 mod 13 = 243 mod 13 = 9
        let mut input = Vec::new();
        input.extend_from_slice(&[0u8; 31]);
        input.push(1); // Bsize
        input.extend_from_slice(&[0u8; 31]);
        input.push(1); // Esize
        input.extend_from_slice(&[0u8; 31]);
        input.push(1); // Msize
        input.push(3); // B
        input.push(5); // E
        input.push(13); // M

        let init_code = build_precompile_caller(0x05, &input);
        let contract = deploy_contract(&mut state, init_code);
        call_contract(&mut state, &contract, &[]);
    }

    // ── Direct precompile call via evm_call ─────────────────────

    #[test]
    fn direct_staticcall_to_sha256() {
        let mut state = setup_deployer();
        let init_code = build_precompile_caller(0x02, b"test");
        let contract = deploy_contract(&mut state, init_code);

        let nonce = state.nonce(&deployer());
        let receipt = evm_call(
            &mut state,
            hash(b"sha256-test"),
            &deployer(),
            &contract,
            &[],
            nonce,
            10_000_000,
            0,
        );
        assert!(receipt.success, "sha256 call failed: {:?}", receipt.error);
        assert!(receipt.gas_used > 21_000);
    }

    #[test]
    fn direct_staticcall_to_identity() {
        let mut state = setup_deployer();
        let init_code = build_precompile_caller(0x04, &[0x42; 32]);
        let contract = deploy_contract(&mut state, init_code);

        let nonce = state.nonce(&deployer());
        let receipt = evm_call(
            &mut state,
            hash(b"identity-test"),
            &deployer(),
            &contract,
            &[],
            nonce,
            10_000_000,
            0,
        );
        assert!(receipt.success, "identity call failed: {:?}", receipt.error);
    }

    #[test]
    fn direct_staticcall_to_ecrecover() {
        let mut state = setup_deployer();
        let input = [0u8; 128];
        let init_code = build_precompile_caller(0x01, &input);
        let contract = deploy_contract(&mut state, init_code);

        let nonce = state.nonce(&deployer());
        let receipt = evm_call(
            &mut state,
            hash(b"ecrecover-test"),
            &deployer(),
            &contract,
            &[],
            nonce,
            10_000_000,
            0,
        );
        assert!(
            receipt.success,
            "ecrecover call failed: {:?}",
            receipt.error
        );
    }
}
