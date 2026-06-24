//! Uniswap V2-style swap workload: a realistic DeFi composite that exercises
//! the `tx -> router -> token.transferFrom / pair.swap -> token.transfer` call
//! graph across four contracts.
//!
//! Setup (folded into genesis, never in blocks): deploy two `TestToken`s, the
//! pair and the router; seed the pair with liquidity; and give every sender an
//! input-token balance plus a router allowance. Each produced transaction is a
//! single router swap.
//!
//! The pair and router bytecode is compiled from
//! `fixtures/contracts/uniswap_v2_min/UniswapV2Min.sol` with solc 0.8.31
//! (`solc --optimize --optimize-runs 200 --bin UniswapV2Min.sol`). The tokens
//! reuse the `TestToken` ERC20 fixture.

use ethrex_common::{
    Address, U256, evm::calculate_create_address, types::Transaction, types::TxKind,
};
use ethrex_l2_common::calldata::Value;
use ethrex_sdk::calldata::encode_calldata;
use eyre::OptionExt;

use super::{GenCtx, build_eip1559, erc20::TEST_TOKEN_INITCODE};

/// `UniswapV2MinPair` creation bytecode (takes `(address token0, address token1)`).
const PAIR_INITCODE: &str = "6080604052348015600e575f5ffd5b506040516107f03803806107f0833981016040819052602b916074565b5f80546001600160a01b039384166001600160a01b0319918216179091556001805492909316911617905560a0565b80516001600160a01b0381168114606f575f5ffd5b919050565b5f5f604083850312156084575f5ffd5b608b83605a565b9150609760208401605a565b90509250929050565b610743806100ad5f395ff3fe608060405234801561000f575f5ffd5b5060043610610055575f3560e01c80630902f1ac146100595780630dfe1681146100885780631249c58b146100b25780636d9a640a146100bc578063d21220a7146100cf575b5f5ffd5b600254604080516001600160701b038084168252600160701b9093049092166020830152015b60405180910390f35b5f5461009a906001600160a01b031681565b6040516001600160a01b03909116815260200161007f565b6100ba6100e2565b005b6100ba6100ca36600461064b565b610204565b60015461009a906001600160a01b031681565b5f546040516370a0823160e01b81523060048201526001600160a01b03909116906370a0823190602401602060405180830381865afa158015610127573d5f5f3e3d5ffd5b505050506040513d601f19601f8201168201806040525081019061014b919061068c565b600280546dffffffffffffffffffffffffffff19166001600160701b03929092169190911790556001546040516370a0823160e01b81523060048201526001600160a01b03909116906370a0823190602401602060405180830381865afa1580156101b8573d5f5f3e3d5ffd5b505050506040513d601f19601f820116820180604052508101906101dc919061068c565b6002600e6101000a8154816001600160701b0302191690836001600160701b03160217905550565b5f83118061021157505f82115b6102585760405162461bcd60e51b8152602060048201526013602482015272125394d551919250d251539517d3d555141555606a1b60448201526064015b60405180910390fd5b6002546001600160701b0380821691600160701b90041681851080156102865750806001600160701b031684105b6102cb5760405162461bcd60e51b8152602060048201526016602482015275494e53554646494349454e545f4c495155494449545960501b604482015260640161024f565b8415610346575f5460405163a9059cbb60e01b81526001600160a01b038581166004830152602482018890529091169063a9059cbb906044016020604051808303815f875af1158015610320573d5f5f3e3d5ffd5b505050506040513d601f19601f8201168201806040525081019061034491906106a3565b505b83156103c25760015460405163a9059cbb60e01b81526001600160a01b038581166004830152602482018790529091169063a9059cbb906044016020604051808303815f875af115801561039c573d5f5f3e3d5ffd5b505050506040513d601f19601f820116820180604052508101906103c091906106a3565b505b5f80546040516370a0823160e01b81523060048201526001600160a01b03909116906370a0823190602401602060405180830381865afa158015610408573d5f5f3e3d5ffd5b505050506040513d601f19601f8201168201806040525081019061042c919061068c565b6001546040516370a0823160e01b81523060048201529192505f916001600160a01b03909116906370a0823190602401602060405180830381865afa158015610477573d5f5f3e3d5ffd5b505050506040513d601f19601f8201168201806040525081019061049b919061068c565b90505f6104b1886001600160701b0387166106dd565b83116104bd575f6104da565b6104d0886001600160701b0387166106dd565b6104da90846106dd565b90505f6104f0886001600160701b0387166106dd565b83116104fc575f610519565b61050f886001600160701b0387166106dd565b61051990846106dd565b90505f82118061052857505f81115b6105695760405162461bcd60e51b8152602060048201526012602482015271125394d551919250d251539517d25394155560721b604482015260640161024f565b5f6105758360036106f6565b610581866103e86106f6565b61058b91906106dd565b90505f6105998360036106f6565b6105a5866103e86106f6565b6105af91906106dd565b90506105c76001600160701b03808916908a166106f6565b6105d490620f42406106f6565b6105de82846106f6565b10156106105760405162461bcd60e51b81526020600482015260016024820152604b60f81b604482015260640161024f565b5050600280546001600160701b03948516600160701b026001600160e01b031990911694909516939093179390931790915550505050505050565b5f5f5f6060848603121561065d575f5ffd5b833592506020840135915060408401356001600160a01b0381168114610681575f5ffd5b809150509250925092565b5f6020828403121561069c575f5ffd5b5051919050565b5f602082840312156106b3575f5ffd5b815180151581146106c2575f5ffd5b9392505050565b634e487b7160e01b5f52601160045260245ffd5b818103818111156106f0576106f06106c9565b92915050565b80820281158282048414176106f0576106f06106c956fea26469706673582212208ca596ff7917c0cd7109a92882f6563a85d4c9b9a88f5e0af9f9038f22cc1bec64736f6c634300081f0033";
/// `UniswapV2MinRouter` creation bytecode (no constructor arguments).
const ROUTER_INITCODE: &str = "6080604052348015600e575f5ffd5b506104e28061001c5f395ff3fe608060405234801561000f575f5ffd5b5060043610610034575f3560e01c8063054d50d414610038578063ad4207c81461005d575b5f5ffd5b61004b61004636600461032c565b610072565b60405190815260200160405180910390f35b61007061006b36600461036c565b6100be565b005b5f80610080856103e56103d0565b90505f61008d84836103d0565b90505f8261009d876103e86103d0565b6100a791906103ed565b90506100b38183610400565b979650505050505050565b6040516323b872dd60e01b81523360048201526001600160a01b038581166024830152604482018490528416906323b872dd906064016020604051808303815f875af1158015610110573d5f5f3e3d5ffd5b505050506040513d601f19601f82011682018060405250810190610134919061041f565b505f5f856001600160a01b0316630902f1ac6040518163ffffffff1660e01b81526004016040805180830381865afa158015610172573d5f5f3e3d5ffd5b505050506040513d601f19601f820116820180604052508101906101969190610460565b91509150856001600160a01b0316630dfe16816040518163ffffffff1660e01b8152600401602060405180830381865afa1580156101d6573d5f5f3e3d5ffd5b505050506040513d601f19601f820116820180604052508101906101fa9190610491565b6001600160a01b0316856001600160a01b03160361029d575f61023085846001600160701b0316846001600160701b0316610072565b6040516336cd320560e11b81525f6004820152602481018290526001600160a01b03868116604483015291925090881690636d9a640a906064015f604051808303815f87803b158015610281575f5ffd5b505af1158015610293573d5f5f3e3d5ffd5b5050505050610324565b5f6102bb85836001600160701b0316856001600160701b0316610072565b6040516336cd320560e11b8152600481018290525f60248201526001600160a01b03868116604483015291925090881690636d9a640a906064015f604051808303815f87803b15801561030c575f5ffd5b505af115801561031e573d5f5f3e3d5ffd5b50505050505b505050505050565b5f5f5f6060848603121561033e575f5ffd5b505081359360208301359350604090920135919050565b6001600160a01b0381168114610369575f5ffd5b50565b5f5f5f5f6080858703121561037f575f5ffd5b843561038a81610355565b9350602085013561039a81610355565b92506040850135915060608501356103b181610355565b939692955090935050565b634e487b7160e01b5f52601160045260245ffd5b80820281158282048414176103e7576103e76103bc565b92915050565b808201808211156103e7576103e76103bc565b5f8261041a57634e487b7160e01b5f52601260045260245ffd5b500490565b5f6020828403121561042f575f5ffd5b8151801515811461043e575f5ffd5b9392505050565b80516001600160701b038116811461045b575f5ffd5b919050565b5f5f60408385031215610471575f5ffd5b61047a83610445565b915061048860208401610445565b90509250929050565b5f602082840312156104a1575f5ffd5b815161043e8161035556fea264697066735822122043e5ecc9354c22f12bfded9471a1123545e4c1f261564adca26c63acce7e376c64736f6c634300081f0033";

/// Liquidity seeded into the pair for each token (10^21 = 1000 tokens).
const LIQUIDITY: u128 = 1_000_000_000_000_000_000_000;
/// Per-swap input amount (10^15), tiny relative to the reserves.
const SWAP_AMOUNT_IN: u128 = 1_000_000_000_000_000;

const DEPLOY_TX_GAS: u64 = 5_000_000;
const SWAP_TX_GAS: u64 = 300_000;

#[derive(Clone)]
pub struct UniswapDeployment {
    pub router: Address,
    /// Input token (token0 of the pair).
    pub token_a: Address,
    /// Output token (token1 of the pair).
    pub token_b: Address,
    pub pair: Address,
}

pub async fn setup_txs(ctx: &mut GenCtx) -> eyre::Result<Vec<Transaction>> {
    let chain_id = ctx.chain_id;
    let senders = ctx.senders().to_vec();
    let deployer = senders[0].clone();
    let deployer_addr = deployer.address();

    // Deterministic deploy addresses from the deployer's first four nonces.
    let token_a = calculate_create_address(deployer_addr, 0);
    let token_b = calculate_create_address(deployer_addr, 1);
    let pair = calculate_create_address(deployer_addr, 2);
    let router = calculate_create_address(deployer_addr, 3);
    ctx.uniswap = Some(UniswapDeployment {
        router,
        token_a,
        token_b,
        pair,
    });

    let token_initcode = hex::decode(TEST_TOKEN_INITCODE)
        .map_err(|err| eyre::eyre!("invalid token initcode: {err}"))?;
    let pair_initcode = {
        let mut code = hex::decode(PAIR_INITCODE)
            .map_err(|err| eyre::eyre!("invalid pair initcode: {err}"))?;
        code.extend_from_slice(&abi_address(token_a));
        code.extend_from_slice(&abi_address(token_b));
        code
    };
    let router_initcode = hex::decode(ROUTER_INITCODE)
        .map_err(|err| eyre::eyre!("invalid router initcode: {err}"))?;

    // Per-sender nonce tracking for the setup transactions.
    let mut nonces = vec![0u64; senders.len()];
    let mut txs = Vec::new();
    let liquidity = U256::from(LIQUIDITY);

    // Deployer: deploy the four contracts (nonces 0..=3).
    txs.push(deploy(&deployer, &mut nonces[0], token_initcode.clone(), chain_id).await?);
    txs.push(deploy(&deployer, &mut nonces[0], token_initcode.clone(), chain_id).await?);
    txs.push(deploy(&deployer, &mut nonces[0], pair_initcode, chain_id).await?);
    txs.push(deploy(&deployer, &mut nonces[0], router_initcode, chain_id).await?);

    // Deployer: mint both tokens and seed the pair, then register reserves.
    let free_mint = encode_calldata("freeMint()", &[])?;
    let mint_pair = encode_calldata("mint()", &[])?;
    txs.push(
        call(
            &deployer,
            &mut nonces[0],
            token_a,
            free_mint.clone(),
            chain_id,
        )
        .await?,
    );
    txs.push(
        call(
            &deployer,
            &mut nonces[0],
            token_b,
            free_mint.clone(),
            chain_id,
        )
        .await?,
    );
    txs.push(
        call(
            &deployer,
            &mut nonces[0],
            token_a,
            encode_calldata(
                "transfer(address,uint256)",
                &[Value::Address(pair), Value::Uint(liquidity)],
            )?,
            chain_id,
        )
        .await?,
    );
    txs.push(
        call(
            &deployer,
            &mut nonces[0],
            token_b,
            encode_calldata(
                "transfer(address,uint256)",
                &[Value::Address(pair), Value::Uint(liquidity)],
            )?,
            chain_id,
        )
        .await?,
    );
    txs.push(call(&deployer, &mut nonces[0], pair, mint_pair, chain_id).await?);

    // Every sender: get an input-token balance and approve the router.
    let approve = encode_calldata(
        "approve(address,uint256)",
        &[Value::Address(router), Value::Uint(U256::MAX)],
    )?;
    for (i, sender) in senders.iter().enumerate() {
        txs.push(call(sender, &mut nonces[i], token_a, free_mint.clone(), chain_id).await?);
        txs.push(call(sender, &mut nonces[i], token_a, approve.clone(), chain_id).await?);
    }

    Ok(txs)
}

pub async fn next_tx(ctx: &mut GenCtx) -> eyre::Result<Transaction> {
    let deployment = ctx
        .uniswap
        .clone()
        .ok_or_eyre("uniswap deployment not set: workload setup did not run")?;
    let (signer, nonce) = ctx.take_sender();
    let calldata = encode_calldata(
        "swapExactTokensForTokens(address,address,uint256,address)",
        &[
            Value::Address(deployment.pair),
            Value::Address(deployment.token_a),
            Value::Uint(U256::from(SWAP_AMOUNT_IN)),
            Value::Address(signer.address()),
        ],
    )?;
    build_eip1559(
        nonce,
        U256::zero(),
        calldata,
        TxKind::Call(deployment.router),
        SWAP_TX_GAS,
        &signer,
        ctx.chain_id,
    )
    .await
}

async fn deploy(
    signer: &ethrex_l2_rpc::signer::Signer,
    nonce: &mut u64,
    initcode: Vec<u8>,
    chain_id: u64,
) -> eyre::Result<Transaction> {
    let tx = build_eip1559(
        *nonce,
        U256::zero(),
        initcode,
        TxKind::Create,
        DEPLOY_TX_GAS,
        signer,
        chain_id,
    )
    .await?;
    *nonce += 1;
    Ok(tx)
}

async fn call(
    signer: &ethrex_l2_rpc::signer::Signer,
    nonce: &mut u64,
    to: Address,
    calldata: Vec<u8>,
    chain_id: u64,
) -> eyre::Result<Transaction> {
    let tx = build_eip1559(
        *nonce,
        U256::zero(),
        calldata,
        TxKind::Call(to),
        DEPLOY_TX_GAS,
        signer,
        chain_id,
    )
    .await?;
    *nonce += 1;
    Ok(tx)
}

/// ABI-encode an address as a left-padded 32-byte word.
fn abi_address(address: Address) -> [u8; 32] {
    let mut word = [0u8; 32];
    word[12..].copy_from_slice(address.as_bytes());
    word
}
