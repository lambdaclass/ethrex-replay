use ethrex_common::{
    Address, U256,
    types::{EIP1559Transaction, Transaction, TxKind},
};
use ethrex_l2_common::calldata::Value;
use ethrex_l2_rpc::signer::{Signable, Signer};
use ethrex_sdk::calldata::encode_calldata;

pub enum TxBuilder {
    ERC20Transfer(Address),
    ETHTransfer,
}

impl TxBuilder {
    pub async fn build_tx(&self, nonce: u64, signer: &Signer, chain_id: u64) -> Transaction {
        match self {
            TxBuilder::ERC20Transfer(address) => {
                let calldata = encode_calldata(
                    "transfer(address,uint256)",
                    &[Value::Address(Address::random()), Value::Uint(U256::one())],
                )
                .expect("failed to encode ERC20 transfer calldata");

                Self::build_signed_transaction(nonce, 0, calldata, *address, signer, chain_id).await
            }
            TxBuilder::ETHTransfer => {
                Self::build_signed_transaction(
                    nonce,
                    1,
                    Vec::default(),
                    Address::random(),
                    signer,
                    chain_id,
                )
                .await
            }
        }
    }

    async fn build_signed_transaction(
        nonce: u64,
        value: u64,
        calldata: Vec<u8>,
        to: Address,
        signer: &Signer,
        chain_id: u64,
    ) -> Transaction {
        Transaction::EIP1559Transaction(EIP1559Transaction {
            nonce,
            value: U256::from(value),
            gas_limit: 250000,
            max_fee_per_gas: u64::MAX,
            max_priority_fee_per_gas: 10,
            chain_id,
            data: calldata.into(),
            to: TxKind::Call(to),
            ..Default::default()
        })
        .sign(signer)
        .await
        .expect("failed to sign transaction")
    }
}
