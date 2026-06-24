//! Minimal EVM bytecode assembler and builders for the synthetic
//! compute/state workloads.
//!
//! Loop workloads share one template (the EEST `JumpLoopGenerator` shape with
//! a clean gas-bounded exit borrowed from spamoor's gas burner):
//!
//! ```text
//! [prelude]                       one-time setup (memory, calldata cursor)
//! loop_start: JUMPDEST
//!             GAS PUSH2 rem GT ISZERO PUSH2 exit JUMPI   exit when gas is low
//!             [body]              the measured work (stack-neutral)
//!             PUSH2 loop_start JUMP
//! exit:       JUMPDEST STOP       clean exit: the transaction succeeds
//! ```
//!
//! The transaction's gas limit bounds the loop, so each transaction burns
//! `gas_limit - rem` gas. Bodies are stack-neutral, so iteration count is
//! limited only by gas. `rem` is chosen larger than one iteration's cost so
//! the loop never runs out of gas mid-body (which would revert).

// Opcodes used by the builders.
const STOP: u8 = 0x00;
const ADD: u8 = 0x01;
const MULMOD: u8 = 0x09;
const GT: u8 = 0x11;
const ISZERO: u8 = 0x15;
const KECCAK256: u8 = 0x20;
const CALLDATALOAD: u8 = 0x35;
const CODECOPY: u8 = 0x39;
const POP: u8 = 0x50;
const MSTORE: u8 = 0x52;
const SLOAD: u8 = 0x54;
const SSTORE: u8 = 0x55;
const JUMP: u8 = 0x56;
const JUMPI: u8 = 0x57;
const JUMPDEST: u8 = 0x5b;
const GAS: u8 = 0x5a;
const PUSH1: u8 = 0x60;
const DUP1: u8 = 0x80;
const SWAP1: u8 = 0x90;
const STATICCALL: u8 = 0xfa;

/// EIP-170 maximum deployed contract size.
pub const MAX_CODE_SIZE: usize = 24_576;

fn push1(out: &mut Vec<u8>, value: u8) {
    out.push(PUSH1);
    out.push(value);
}

fn push2(out: &mut Vec<u8>, value: u16) {
    out.push(PUSH1 + 1);
    out.extend_from_slice(&value.to_be_bytes());
}

fn push32(out: &mut Vec<u8>, value: [u8; 32]) {
    out.push(PUSH1 + 31);
    out.extend_from_slice(&value);
}

/// Assemble the loop template around a prelude and a stack-neutral body.
fn build_loop_runtime(prelude: &[u8], body: &[u8], remainder: u16) -> Vec<u8> {
    let p0 = prelude.len();
    let b = body.len();
    let loop_start = p0;
    let exit = p0 + 15 + b;
    assert!(
        p0 + 17 + b <= MAX_CODE_SIZE,
        "loop runtime exceeds max code size"
    );

    let mut code = Vec::with_capacity(p0 + 17 + b);
    code.extend_from_slice(prelude);
    code.push(JUMPDEST); // loop_start
    // Push `remainder` first so that GT compares gasleft (top of stack)
    // against it: GT yields `gasleft > remainder`.
    push2(&mut code, remainder);
    code.push(GAS);
    code.push(GT); // gasleft > remainder
    code.push(ISZERO); // true when gasleft <= remainder
    push2(&mut code, exit as u16);
    code.push(JUMPI); // exit the loop when gas is low

    code.extend_from_slice(body);
    push2(&mut code, loop_start as u16);
    code.push(JUMP);
    code.push(JUMPDEST); // exit
    code.push(STOP);
    code
}

fn repeat(core: &[u8], times: usize) -> Vec<u8> {
    core.repeat(times)
}

/// `keccak`: hash a 136-byte (one-permutation) memory region each iteration.
pub fn keccak_runtime() -> Vec<u8> {
    let mut core = Vec::new();
    push1(&mut core, 0x88); // size = 136
    push1(&mut core, 0x00); // offset = 0
    core.push(KECCAK256);
    core.push(POP);
    build_loop_runtime(&[], &repeat(&core, 50), 10_000)
}

/// `mulmod`: 256-bit modular multiplication with worst-case operands.
pub fn mulmod_runtime() -> Vec<u8> {
    // a = b = 2^256 - 1 (maximal operands); n = 2^256 - 189 (large, odd) so
    // the result never collapses to a cheap path or to zero.
    let max = [0xffu8; 32];
    let mut n = [0xffu8; 32];
    n[31] = 0xff - 188; // 0x43

    let mut core = Vec::new();
    push32(&mut core, max);
    push32(&mut core, max);
    push32(&mut core, n);
    core.push(MULMOD);
    core.push(POP);
    build_loop_runtime(&[], &repeat(&core, 50), 10_000)
}

/// A valid secp256k1 signature vector (go-ethereum's canonical ECRECOVER test
/// vector). `ECRECOVER_VECTOR` recovers to `ECRECOVER_EXPECTED`, so the
/// precompile does real work instead of early-returning empty output.
pub const ECRECOVER_HASH: [u8; 32] =
    hex_literal(b"456e9aea5e197a1f1af7a3e85a3212fa4049a3ba34c2289b4c860fc0b0c64ef3");
pub const ECRECOVER_V: u8 = 28;
pub const ECRECOVER_R: [u8; 32] =
    hex_literal(b"9242685bf161793cc25603c231bc2f568eb630ea16aa137d2664ac8038825608");
pub const ECRECOVER_S: [u8; 32] =
    hex_literal(b"4f8ae3bd7535248d0bd448298cc2e2071e56992d0774dc340c368ae950852ada");
pub const ECRECOVER_EXPECTED: [u8; 20] =
    hex_literal_20(b"7156526fbd7a3c72969b54f64e42c10fbb768c8a");

/// `ecrecover`: `STATICCALL` to precompile 0x01 with a valid signature.
pub fn ecrecover_runtime() -> Vec<u8> {
    // Prelude: lay out hash ‖ v ‖ r ‖ s in memory[0..128].
    let mut prelude = Vec::new();
    push32(&mut prelude, ECRECOVER_HASH);
    push1(&mut prelude, 0x00);
    prelude.push(MSTORE);
    push1(&mut prelude, ECRECOVER_V);
    push1(&mut prelude, 0x20);
    prelude.push(MSTORE);
    push32(&mut prelude, ECRECOVER_R);
    push1(&mut prelude, 0x40);
    prelude.push(MSTORE);
    push32(&mut prelude, ECRECOVER_S);
    push1(&mut prelude, 0x60);
    prelude.push(MSTORE);

    // Body: STATICCALL(gas=5000, addr=1, in=0, insize=128, out=128, outsize=32).
    let mut core = Vec::new();
    push1(&mut core, 0x20); // retSize
    push1(&mut core, 0x80); // retOffset (after the 128-byte input)
    push1(&mut core, 0x80); // argsSize
    push1(&mut core, 0x00); // argsOffset
    push1(&mut core, 0x01); // precompile address
    push2(&mut core, 5000); // forwarded gas
    core.push(STATICCALL);
    core.push(POP);
    build_loop_runtime(&prelude, &core, 10_000)
}

/// `sstore-fresh`: write a non-zero value to a fresh keccak-spread slot each
/// iteration. The starting key comes from calldata so successive transactions
/// (and blocks) write disjoint slots, maximising state growth.
pub fn sstore_fresh_runtime() -> Vec<u8> {
    let mut prelude = Vec::new();
    push1(&mut prelude, 0x00);
    prelude.push(CALLDATALOAD); // counter = calldata[0..32]

    // Body (stack: [counter]): SSTORE(keccak(counter), 1), counter += 1.
    let mut core = Vec::new();
    core.push(DUP1);
    push1(&mut core, 0x00);
    core.push(MSTORE); // mem[0] = counter
    push1(&mut core, 0x20);
    push1(&mut core, 0x00);
    core.push(KECCAK256); // key = keccak(counter)
    push1(&mut core, 0x01); // value = 1
    core.push(SWAP1); // [counter, value, key]
    core.push(SSTORE);
    push1(&mut core, 0x01);
    core.push(ADD); // counter += 1
    // A cold fresh SSTORE costs ~22.1k; keep a wide margin.
    build_loop_runtime(&prelude, &core, 50_000)
}

/// `sload-cold`: read sequential cold slots starting at a calldata-provided
/// base. The base advances per transaction so reads stay cold and distinct.
pub fn sload_cold_runtime() -> Vec<u8> {
    let mut prelude = Vec::new();
    push1(&mut prelude, 0x00);
    prelude.push(CALLDATALOAD); // counter = calldata[0..32]

    // Body (stack: [counter]): SLOAD(counter), counter += 1.
    let mut core = Vec::new();
    core.push(DUP1);
    core.push(SLOAD);
    core.push(POP);
    push1(&mut core, 0x01);
    core.push(ADD);
    build_loop_runtime(&prelude, &core, 10_000)
}

/// Initcode that deploys `code_size` bytes of runtime (all `STOP`). Used by
/// the `contract-deploy` workload to exercise CREATE and code-deposit costs.
pub fn deploy_initcode(code_size: usize) -> Vec<u8> {
    assert!(code_size <= MAX_CODE_SIZE, "deploy code size exceeds limit");
    let mut code = Vec::new();
    // PUSH2 len; PUSH2 15; PUSH1 0; CODECOPY; PUSH2 len; PUSH1 0; RETURN
    push2(&mut code, code_size as u16);
    push2(&mut code, 15); // runtime starts after this 15-byte prelude
    push1(&mut code, 0x00);
    code.push(CODECOPY);
    push2(&mut code, code_size as u16);
    push1(&mut code, 0x00);
    code.push(0xf3); // RETURN
    debug_assert_eq!(code.len(), 15);
    code.extend(std::iter::repeat_n(STOP, code_size));
    code
}

const fn hex_nibble(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => panic!("invalid hex digit"),
    }
}

/// Decode a 64-character hex string into a 32-byte array at compile time.
const fn hex_literal(hex: &[u8]) -> [u8; 32] {
    assert!(hex.len() == 64, "expected 64 hex characters");
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = (hex_nibble(hex[i * 2]) << 4) | hex_nibble(hex[i * 2 + 1]);
        i += 1;
    }
    out
}

/// Decode a 40-character hex string into a 20-byte array at compile time.
const fn hex_literal_20(hex: &[u8]) -> [u8; 20] {
    assert!(hex.len() == 40, "expected 40 hex characters");
    let mut out = [0u8; 20];
    let mut i = 0;
    while i < 20 {
        out[i] = (hex_nibble(hex[i * 2]) << 4) | hex_nibble(hex[i * 2 + 1]);
        i += 1;
    }
    out
}
