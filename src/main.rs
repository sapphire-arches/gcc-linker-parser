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

    println!("{:?} {:?}", a_maps, b_maps);

    let mut a_maps = symbol_sizes(a_maps.into()).context("Parse baseline")?;
    let mut b_maps = symbol_sizes(b_maps.into()).context("Parse new")?;

    let a_symbols = a_maps.compute_symbol_sizes();
    let b_symbols = b_maps.compute_symbol_sizes();

    let mut all_symbol_names: Vec<_> = a_symbols.keys().chain(b_symbols.keys()).collect();
    all_symbol_names.sort();
    all_symbol_names.dedup();

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

    let a_total_size: u64 = a_symbols.values().sum();
    let b_total_size: u64 = b_symbols.values().sum();

    println!(
        "Total size delta: {} -> {}  change: {}",
        a_total_size,
        b_total_size,
        b_total_size as i64 - a_total_size as i64
    );
    println!(
        "Padding delta: {} -> {}  change: {}",
        a_maps.padding,
        b_maps.padding,
        b_maps.padding as i64 - a_maps.padding as i64
    );

    Ok(())
}
