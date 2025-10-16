#[cfg(not(feature = "l2"))]
use ethrex_config::networks::Network;
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
