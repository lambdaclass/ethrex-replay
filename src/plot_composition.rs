use ethrex_common::types::{Block, Transaction, TxKind, TxType};
use ethrex_common::U256;
use std::collections::HashMap;
use std::path::Path;
use tracing::info;

use charming::{
    Chart, ImageRenderer,
    component::Legend,
    element::{Tooltip, Trigger},
    series::Pie,
};

const TOP_N_DESTINATIONS: usize = 10;
const TOP_N_SELECTORS: usize = 10;

fn categorize_selector(sel: [u8; 4]) -> String {
    let selector = hex::encode(sel);
    match selector.as_str() {
        "a9059cbb" => "transfer",
        "095ea7b3" => "approve",
        "3593564c" => "swap", // execute(bytes,bytes[],uint256)
        "5f575529" => "swap",
        "2213bc0b" => "swap", // exec(address,address,uint256,address,bytes)
        "b6f9de95" => "swap",
        "1f6a1eb9" => "exec", // execute(bytes,bytes)
        "791ac947" => "swap",
        "23b872dd" => "transfer",
        "c46b30bc" => "swap",
        "0162e2d0" => "swap",
        "12aa3caf" => "swap",
        "78e111f6" => "mevbot", // executeFFsYo
        "088890dc" => "swap",
        "3d0e3ec5" => "swap",
        "9871efa4" => "swap",
        "c14c9204" => "oracle",
        "7ff36ab5" => "swap",
        "deff4b24" => "relay",
        "0d5f0e3b" => "swap",
        "b143044b" => "exec", // execute(ExecuteParam[])
        "d9caed12" => "withdraw",
        "6a761202" => "exec",
        "e63d38ed" => "mass transfer",
        "fb3bdb41" => "swap",
        "049639fb" => "exec",
        "d0e30db0" => "deposit",
        "18cbafe5" => "swap",
        "7b939232" => "deposit",
        "0894edf1" => "relay",       // commitVerification(bytes,bytes32)
        "28832cbd" => "swap&bridge", // swapAndStartBridgeTokensViaAcrossV3(...)
        "07ed2379" => "swap",
        "e9ae5c53" => "exec",
        "0cf79e0a" => "swap",
        "c7a76969" => "swap", // strictlySwapAndCallDln(...)
        "4782f779" => "withdraw",
        "2c65169e" => "swap", // buyWithEth(uint256,bool)
        "3ce33bff" => "bridge",
        "0dcd7a6c" => "transfer", // sendMultiSigToken(...)
        "2c57e884" => "swap",
        "38ed1739" => "swap",
        "b6b55f25" => "deposit",
        "3ccfd60b" => "withdraw",
        "30c48952" => "swap&bridge", // swapAndStartBridgeTokensViaMayan
        "13d79a0b" => "swap",        // settle
        _ => &selector,
    }
    .to_string()
}

fn known_contract_name(addr: &str) -> Option<&'static str> {
    match addr {
        "0xdac17f958d2ee523a2206206994597c13d831ec7" => Some("USDT"),
        "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" => Some("USDC"),
        "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2" => Some("WETH"),
        "0x6b175474e89094c44da98b954eedeac495271d0f" => Some("DAI"),
        "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599" => Some("WBTC"),
        "0x514910771af9ca656af840dff83e8264ecf986ca" => Some("LINK"),
        "0x1f9840a85d5af5bf1d1762f925bdaddc4201f984" => Some("UNI"),
        "0x7d1afa7b718fb893db30a3abc0cfc608aacfebb0" => Some("MATIC"),
        "0x95ad61b0a150d79219dcf64e1e6cc01f0b64c4ce" => Some("SHIB"),
        "0xae7ab96520de3a18e5e111b5eaab095312d7fe84" => Some("stETH (Lido)"),
        "0x7fc66500c84a76ad7e9c93437bfc5ac33e2ddae9" => Some("AAVE"),
        "0xe592427a0aece92de3edee1f18e0157c05861564" => Some("Uniswap V3 Router"),
        "0x7a250d5630b4cf539739df2c5dacb4c659f2488d" => Some("Uniswap V2 Router"),
        "0x68b3465833fb72a70ecdf485e0e4c7bd8665fc45" => Some("Uniswap Universal Router"),
        "0x3fc91a3afd70395cd496c647d5a6cc9d4b2b7fad" => Some("Uniswap Universal Router 2"),
        "0xef1c6e67703c7bd7107eed8303fbe6ec2554bf6b" => Some("Uniswap Universal Router (old)"),
        "0x1111111254eeb25477b68fb85ed929f73a960582" => Some("1inch V5 Router"),
        "0x1111111254fb6c44bac0bed2854e76f90643097d" => Some("1inch V4 Router"),
        "0x881d40237659c251811cec9c364ef91dc08d300c" => Some("Metamask Swap Router"),
        "0xdef1c0ded9bec7f1a1670819833240f027b25eff" => Some("0x Exchange Proxy"),
        "0x00000000006c3852cbef3e08e8df289169ede581" => Some("OpenSea Seaport"),
        "0x00000000000000adc04c56bf30ac9d3c0aaf14dc" => Some("OpenSea Seaport 1.5"),
        "0xd9e1ce17f2641f24ae83637ab66a2cca9c378b9f" => Some("SushiSwap Router"),
        "0x87870bca3f3fd6335c3f4ce8392d69350b4fa4e2" => Some("Aave V3 Pool"),
        "0x7d2768de32b0b80b7a3454c06bdac94a69ddc7a9" => Some("Aave V2 Pool"),
        "0xae78736cd615f374d3085123a210448e74fc6393" => Some("rETH (Rocket Pool)"),
        "0xbe9895146f7af43049ca1c1ae358b0541ea49704" => Some("cbETH (Coinbase)"),
        "0xa2e3356610840701bdf5611a53974510ae27e2e1" => Some("wBETH (Binance)"),
        "0x32400084c286cf3e17e7b677ea9583e60a000324" => Some("zkSync Bridge"),
        "0x3154cf16ccdb4c6d922629664174b904d80f2c35" => Some("Base Bridge"),
        _ => None,
    }
}

fn tx_type_label(tx_type: TxType) -> String {
    match tx_type {
        TxType::Legacy => "Legacy (Type 0)".to_string(),
        TxType::EIP2930 => "EIP-2930 (Type 1)".to_string(),
        TxType::EIP1559 => "EIP-1559 (Type 2)".to_string(),
        TxType::EIP4844 => "EIP-4844 (Type 3)".to_string(),
        TxType::EIP7702 => "EIP-7702 (Type 4)".to_string(),
        TxType::FeeToken => "FeeToken (Type 0x7d)".to_string(),
        TxType::Privileged => "Privileged (Type 0x7e)".to_string(),
    }
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn shorten_address(addr: &str) -> String {
    if addr.len() > 12 {
        format!("{}...{}", &addr[..6], &addr[addr.len() - 4..])
    } else {
        addr.to_string()
    }
}

fn format_destination(addr: &str) -> String {
    let short = shorten_address(addr);
    match known_contract_name(addr) {
        Some(name) => format!("{name} ({short})"),
        None => short,
    }
}

#[derive(Default, Debug)]
struct BlockCompositionStats {
    destinations: HashMap<String, u64>,
    selector_count: HashMap<String, u64>,
    selector_by_gas: HashMap<String, u64>,
    tx_type_count: HashMap<String, u64>,
    call_category_count: HashMap<String, u64>,
    total_gas_used: u64,
    total_gas_limit: u64,
    total_tx_count: u64,
    block_count: u64,
}

impl BlockCompositionStats {
    fn process_block(&mut self, block: &Block) {
        self.total_gas_used += block.header.gas_used;
        self.total_gas_limit += block.header.gas_limit;
        self.total_tx_count += block.body.transactions.len() as u64;
        self.block_count += 1;

        for tx in &block.body.transactions {
            self.process_tx(tx);
        }
    }

    fn process_tx(&mut self, tx: &Transaction) {
        let label = tx_type_label(tx.tx_type());
        *self.tx_type_count.entry(label).or_insert(0) += 1;

        let category = match tx.to() {
            TxKind::Create => "Contract Creation",
            TxKind::Call(_) => {
                if tx.data().len() >= 4 {
                    "Contract Call"
                } else if tx.value() > U256::zero() {
                    "ETH Transfer"
                } else {
                    "Zero-value Transfer"
                }
            }
        };
        *self
            .call_category_count
            .entry(category.to_string())
            .or_insert(0) += 1;

        if let TxKind::Call(addr) = tx.to() {
            let addr_str = format!("0x{addr:x}");
            let display_name = format_destination(&addr_str);
            *self.destinations.entry(display_name).or_insert(0) += 1;

            if tx.data().len() >= 4 {
                let mut selector = [0u8; 4];
                selector.copy_from_slice(&tx.data()[0..4]);
                let sel_name = categorize_selector(selector);
                *self.selector_count.entry(sel_name.clone()).or_insert(0) += 1;
                *self.selector_by_gas.entry(sel_name).or_insert(0) += tx.gas_limit();
            }
        }
    }

    fn print_summary(&self, first_block: u64, last_block: u64) {
        if first_block == last_block {
            println!(
                "\n=== Block Composition: #{} — {} transactions ===\n",
                format_number(first_block),
                format_number(self.total_tx_count),
            );
        } else {
            println!(
                "\n=== Block Composition: #{} - #{} ({} blocks) — {} transactions ===\n",
                format_number(first_block),
                format_number(last_block),
                format_number(self.block_count),
                format_number(self.total_tx_count),
            );
        }

        println!("--- Gas Summary ---");
        println!(
            "  Total Gas Used     {:>16}",
            format_number(self.total_gas_used)
        );
        println!(
            "  Total Gas Limit    {:>16}",
            format_number(self.total_gas_limit)
        );
        if self.total_gas_limit > 0 {
            let utilization =
                (self.total_gas_used as f64 / self.total_gas_limit as f64) * 100.0;
            println!("  Utilization        {:>15.1}%", utilization);
        }
        println!();

        print_ranked_section(
            "Transaction Types",
            &self.tx_type_count,
            self.total_tx_count,
            None,
        );
        print_ranked_section(
            "Call Categories",
            &self.call_category_count,
            self.total_tx_count,
            None,
        );
        print_ranked_section(
            "Top Method Selectors (by count)",
            &self.selector_count,
            self.total_tx_count,
            Some(TOP_N_SELECTORS),
        );
        print_ranked_section(
            "Top Method Selectors (by gas)",
            &self.selector_by_gas,
            self.total_gas_used,
            Some(TOP_N_SELECTORS),
        );
        print_ranked_section(
            "Top Destinations",
            &self.destinations,
            self.total_tx_count,
            Some(TOP_N_DESTINATIONS),
        );
    }

    fn charts(&self) -> Vec<(String, Chart)> {
        let selectors = sorted_desc(&self.selector_count);
        let selectors_by_gas = sorted_desc(&self.selector_by_gas);
        let destinations = sorted_desc(&self.destinations);
        let tx_types = sorted_desc(&self.tx_type_count);
        let call_categories = sorted_desc(&self.call_category_count);

        vec![
            (
                "selectors".to_string(),
                make_pie_chart(
                    &format!("Top{TOP_N_SELECTORS} selectors"),
                    &truncate_to(&selectors, TOP_N_SELECTORS),
                ),
            ),
            (
                "selectors_by_gas".to_string(),
                make_pie_chart(
                    &format!("Top{TOP_N_SELECTORS} selectors by gas limit"),
                    &truncate_to(&selectors_by_gas, TOP_N_SELECTORS),
                ),
            ),
            (
                "destinations".to_string(),
                make_pie_chart(
                    &format!("Top{TOP_N_DESTINATIONS} destinations"),
                    &truncate_to(&destinations, TOP_N_DESTINATIONS),
                ),
            ),
            (
                "tx_types".to_string(),
                make_pie_chart(
                    "Transaction Types",
                    &truncate_to(&tx_types, tx_types.len()),
                ),
            ),
            (
                "call_categories".to_string(),
                make_pie_chart(
                    "Call Categories",
                    &truncate_to(&call_categories, call_categories.len()),
                ),
            ),
        ]
    }
}

fn sorted_desc(map: &HashMap<String, u64>) -> Vec<(&String, &u64)> {
    let mut v: Vec<_> = map.iter().collect();
    v.sort_by(|(_, a), (_, b)| b.cmp(a));
    v
}

fn print_ranked_section(
    title: &str,
    data: &HashMap<String, u64>,
    total: u64,
    top_n: Option<usize>,
) {
    println!("--- {title} ---");
    let mut entries: Vec<_> = data.iter().collect();
    entries.sort_by(|(_, a), (_, b)| b.cmp(a));

    let limit = top_n.unwrap_or(entries.len());

    for (name, count) in entries.iter().take(limit) {
        let pct = if total > 0 {
            (**count as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "  {:<30} {:>8}  {:>5.1}%",
            name,
            format_number(**count),
            pct
        );
    }

    if entries.len() > limit {
        let shown: u64 = entries.iter().take(limit).map(|(_, c)| **c).sum();
        let rest = total.saturating_sub(shown);
        if rest > 0 {
            let pct = (rest as f64 / total as f64) * 100.0;
            println!(
                "  {:<30} {:>8}  {:>5.1}%",
                "... (other)",
                format_number(rest),
                pct
            );
        }
    }

    println!();
}

fn make_pie_chart(name: &str, data: &[(String, u64)]) -> Chart {
    Chart::new()
        .tooltip(Tooltip::new().trigger(Trigger::Item))
        .legend(Legend::new())
        .series(
            Pie::new()
                .name(name)
                .radius(vec!["40%", "55%"])
                .data(
                    data.iter()
                        .map(|(label, count)| (*count as f64, label.as_str()))
                        .collect(),
                ),
        )
}

fn truncate_to(vec: &[(&String, &u64)], size: usize) -> Vec<(String, u64)> {
    let mut included: u64 = 0;
    let mut res: Vec<(String, u64)> = Vec::new();
    for (item, count) in vec.iter().take(size) {
        included += **count;
        res.push((item.to_string(), **count));
    }
    let total: u64 = vec.iter().map(|(_, c)| **c).sum();
    let other = total.saturating_sub(included);
    if other > 0 {
        res.push(("other".to_string(), other));
    }
    res
}

pub fn analyze_and_display(blocks: &[Block], output_dir: &Path) -> eyre::Result<()> {
    let mut stats = BlockCompositionStats::default();
    for block in blocks {
        stats.process_block(block);
    }

    let first = blocks.first().map(|b| b.header.number).unwrap_or(0);
    let last = blocks.last().map(|b| b.header.number).unwrap_or(0);

    stats.print_summary(first, last);

    if !output_dir.exists() {
        std::fs::create_dir_all(output_dir)?;
    }

    let mut renderer = ImageRenderer::new(1000, 800);
    for (name, chart) in stats.charts() {
        let filename = if first == last {
            format!("chart_{name}_{first}.svg")
        } else {
            format!("chart_{name}_{first}-{last}.svg")
        };
        let path = output_dir.join(&filename);
        info!("Saving chart to: {}", path.display());
        renderer.save(&chart, path.to_str().unwrap_or(&filename))?;
    }

    Ok(())
}
