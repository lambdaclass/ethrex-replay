#[cfg(not(feature = "l2"))]
use ethrex_config::networks::Network;
use ethrex_rlp::encode::RLPEncode;
use ethrex_trie::{InMemoryTrieDB, Nibbles, Node, node::BranchNode};
#[cfg(not(feature = "l2"))]
use std::path::Path;

#[cfg(not(feature = "l2"))]
/// Get block numbers inside the cache directory for a given network.
pub fn get_block_numbers_in_cache_dir(dir: &Path, network: &Network) -> eyre::Result<Vec<u64>> {
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

/// Gets all trie nodes as an array of (Path, RLP Value)
/// It also inserts dummy nodes so that we don't have nodes missing during execution.
/// We want this when we request something that doesn't alter the state.
pub fn get_trie_nodes_with_dummies(in_memory_trie: InMemoryTrieDB) -> Vec<(Nibbles, Vec<u8>)> {
    let mut guard = in_memory_trie.inner.lock().unwrap();
    let dummy_branch = Node::from(BranchNode::default()).encode_to_vec();
    // Dummy Branch nodes injection to the trie in order for execution not to fail when we want to access a missing node
    for (nibbles, _node_rlp) in guard.clone() {
        // Skip nodes that already represent full paths, which are 65 bytes.
        if nibbles.len() > 64 {
            continue;
        }

        for nibble in 0x00u8..=0x0fu8 {
            let mut key = nibbles.clone();
            key.push(nibble);
            guard.entry(key).or_insert(dummy_branch.clone());
        }
    }

    guard
        .iter()
        .map(|(key, value)| (Nibbles::from_hex(key.to_vec()), value.clone()))
        .collect()
}
