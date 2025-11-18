use ethrex_common::{H256, types::BlockHeader};
use ethrex_rlp::{decode::RLPDecode, encode::RLPEncode};
use ethrex_rpc::debug::execution_witness::RpcExecutionWitness;
use ethrex_trie::{InMemoryTrieDB, Nibbles, Node, node::BranchNode};
#[cfg(not(feature = "l2"))]
use ethrex_config::networks::Network;
#[cfg(not(feature = "l2"))]
use std::path::Path;

#[cfg(not(feature = "l2"))]
/// Get block numbers inside the cache directory for a given network.
pub fn get_block_numbers_in_cache_dir(
    dir: &Path,
    network: &Network,
) -> eyre::Result<Vec<u64>> {
    let mut block_numbers = Vec::new();
    let entries = std::fs::read_dir(dir)?;
    let prefix = format!("cache_{}_", network);

    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if file_name.starts_with(&prefix) && file_name.ends_with(".json") {
                let number_part = &file_name[prefix.len()..file_name.len() - 5]; // remove ".json"
                if let Ok(number) = number_part.parse::<u64>() {
                    block_numbers.push(number);
                }
            }
        }
    }

    block_numbers.sort_unstable();
    Ok(block_numbers)
}

/// Compute the initial state root needed to rebuild the state trie from an
/// `RpcExecutionWitness`.
pub fn get_initial_state_root(
    network: &ethrex_config::networks::Network,
    witness: &RpcExecutionWitness,
    first_block_number: u64,
) -> eyre::Result<H256> {
    // For the genesis block, derive the state root directly from the genesis
    // allocation.
    if first_block_number == 0 {
        let genesis = network
            .get_genesis()
            .map_err(|e| eyre::eyre!("Failed to get genesis: {e}"))?;
        return Ok(genesis.compute_state_root());
    }

    let parent_number = first_block_number - 1;

    // Headers in the witness are RLP-encoded. Find the parent header and use
    // its state_root as the initial state root.
    for header_bytes in &witness.headers {
        let header =
            BlockHeader::decode(header_bytes).map_err(|e| {
                eyre::eyre!("Failed to decode block header from witness: {e}")
            })?;
        if header.number == parent_number {
            return Ok(header.state_root);
        }
    }

    Err(eyre::eyre!(
        "Parent block header {parent_number} not found in witness headers"
    ))
}

/// Gets all trie nodes as an array of (Path, RLP Value)
/// It also inserts dummy nodes so that we don't have nodes missing during execution.
/// We want this when we request something that doesn't alter the state.
pub fn get_trie_nodes_with_dummies(in_memory_trie: InMemoryTrieDB) -> Vec<(Nibbles, Vec<u8>)> {
    let node_map = in_memory_trie.inner();
    let mut node_map_guard = node_map.lock().unwrap();
    let dummy_branch = Node::from(BranchNode::default()).encode_to_vec();
    // Dummy Branch nodes injection to the trie in order for execution not to fail when we want to access a missing node
    let nodes_paths: Vec<_> = node_map_guard.keys().cloned().collect();
    for nibbles in nodes_paths {
        // Skip nodes that already represent full paths.
        if nibbles.len() > 64 {
            continue;
        }

        for nibble in 0x00u8..=0x0fu8 {
            let mut key = nibbles.clone();
            key.push(nibble);
            node_map_guard.entry(key).or_insert(dummy_branch.clone());
        }
    }

    node_map_guard
        .iter()
        .map(|(key, value)| (Nibbles::from_hex(key.to_vec()), value.clone()))
        .collect()
}
