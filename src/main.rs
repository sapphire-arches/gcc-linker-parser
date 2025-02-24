use std::{collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use parser::MapFile;

mod parser;

fn symbol_sizes(filename: PathBuf) -> Result<MapFile> {
    let file_contents = std::fs::read_to_string(filename).context("Open input file")?;
    let mut map_file = parser::parse_map_file(&file_contents).context("Parse map file")?;

    Ok(map_file)
}

fn main() -> Result<()> {
    let mut args = std::env::args_os();
    // Skip self name
    args.next();
    let a_maps = args
        .next()
        .ok_or_else(|| anyhow!("Missing baseline file name"))?;
    let b_maps = args
        .next()
        .ok_or_else(|| anyhow!("Missing new file name"))?;

    let a_maps = symbol_sizes(a_maps.into()).context("Parse baseline")?;
    let b_maps = symbol_sizes(b_maps.into()).context("Parse new")?;

    let a_symbols: HashMap<String, u64> = a_maps
        .symbols
        .iter()
        .map(|sym| (sym.name.clone(), sym.size))
        .collect();
    let b_symbols: HashMap<String, u64> = b_maps
        .symbols
        .iter()
        .map(|sym| (sym.name.clone(), sym.size))
        .collect();

    let mut all_symbol_names: Vec<_> = a_symbols.keys().chain(b_symbols.keys()).collect();
    // all_symbol_names.sort();
    all_symbol_names.sort_by_key(|symbol| {
        let symbol: &str = symbol.as_ref();
        let os: i64 = a_symbols.get(symbol).copied().unwrap_or_default() as i64;
        let ns: i64 = b_symbols.get(symbol).copied().unwrap_or_default() as i64;

        ns - os
    });
    all_symbol_names.dedup();

    println!("Symbol\tOld Size\tNew Size\t Delta");
    for symbol in all_symbol_names {
        let symbol_strip = symbol.trim();
        match (a_symbols.get(symbol), b_symbols.get(symbol)) {
            (None, None) => {
                unreachable!();
            }
            (None, Some(ns)) => {
                println!("{symbol_strip}\t0\t{ns}\t{ns}");
            }
            (Some(os), None) => {
                println!("{symbol_strip}\t{os}\t0\t-{os}");
            }
            (Some(os), Some(ns)) => {
                if os > ns {
                    println!("{symbol_strip}\t{os}\t{ns}\t-{}", os - ns);
                } else if ns < os {
                    println!("{symbol_strip}\t{os}\t{ns}\t{}", ns - os);
                }
            }
        }
    }

    /*
    for symbol in all_symbol_names {
        match (a_symbols.get(symbol), b_symbols.get(symbol)) {
            (None, None) => {
                unreachable!();
            }
            (None, Some(ns)) => {
                println!("+ {:16} {}", ns, symbol);
            }
            (Some(os), None) => {
                println!("- {:16} {}", os, symbol);
            }
            (Some(os), Some(ns)) => {
                if os > ns {
                    println!("- {:6} -> {:6} {}", os, ns, symbol);
                } else if ns < os {
                    println!("+ {:6} -> {:6} {}", os, ns, symbol);
                }
            }
        }
    }
    */

    let a_total_size: u64 = a_symbols.values().sum();
    let b_total_size: u64 = b_symbols.values().sum();

    // println!(
    //     "Total size delta: {} -> {}  change: {}",
    //     a_total_size,
    //     b_total_size,
    //     b_total_size as i64 - a_total_size as i64
    // );
    // println!(
    //     "Padding delta: {} -> {}  change: {}",
    //     a_maps.padding,
    //     b_maps.padding,
    //     b_maps.padding as i64 - a_maps.padding as i64
    // );

    Ok(())
}
