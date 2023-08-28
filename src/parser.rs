use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammer.pest"]
struct MapParser;

#[derive(Debug)]
pub(crate) struct Symbol {
    pub location: u64,
    pub name: String,
}

pub(crate) struct MapFile {
    pub symbols: Vec<Symbol>,
    pub padding: u64,
}

impl MapFile {
    pub(crate) fn compute_symbol_sizes(&mut self) -> HashMap<String, u64> {
        self.symbols
            .sort_unstable_by(|a, b| a.location.cmp(&b.location));
        self.symbols
            .windows(2)
            .map(|symbols| {
                let symbol = &symbols[0];
                let next_symbol = &symbols[1];
                let size = next_symbol.location - symbol.location;

                (symbol.name.clone(), size)
            })
            .collect()
    }
}

fn pest_error(span: pest::Span, message: impl Into<String>) -> pest::error::Error<Rule> {
    pest::error::Error::new_from_span(
        pest::error::ErrorVariant::CustomError {
            message: message.into(),
        },
        span,
    )
}

fn process_mmap_section(state: &mut MapFile, contents: Pair<Rule>) -> Result<()> {
    let span = contents.as_span();
    let pair = contents
        .into_inner()
        .next()
        .ok_or_else(|| pest_error(span, "Somehow missing child in mmap_section"))?;

    match pair.as_rule() {
        Rule::mmap_section_glob => Ok(()),
        Rule::mmap_section_fill => {
            for pair in pair.into_inner() {
                match pair.as_rule() {
                    Rule::mmap_section_size => {
                        state.padding +=
                            u64::from_str_radix(pair.as_str().split_at(2).1, 16).unwrap();
                        return Ok(());
                    }
                    _ => {}
                }
            }
            Err(pest_error(span, "Failed to find size").into())
        }
        Rule::mmap_section_with_brackets => Ok(()),
        Rule::mmap_section_with_size => Ok(()),
        Rule::mmap_section_with_address => {
            let mut address = None;
            let mut name = None;

            for pair in pair.into_inner() {
                match pair.as_rule() {
                    Rule::mmap_section_address => {
                        // panic safety: the hex_number rule guarantees we have a number of the
                        // form 0x.... where .... is hex digits
                        address =
                            Some(u64::from_str_radix(pair.as_str().split_at(2).1, 16).unwrap())
                    }
                    Rule::mmap_entry => {
                        if let Some(pair) = pair.into_inner().next() {
                            match pair.as_rule() {
                                Rule::mmap_symbol_name => name = Some(pair.as_str().to_owned()),
                                Rule::object_name | Rule::linker_stubs => {
                                    // This is an overall section header, we don't need to generate
                                    // a symbol for it
                                    return Ok(());
                                }
                                _ => {
                                    return Err(pest_error(
                                        span,
                                        format!(
                                            "Missing symbol name. Found {:?} instead",
                                            pair.as_rule()
                                        ),
                                    )
                                    .into());
                                }
                            }
                        }
                    }
                    Rule::mmap_symbol_name => name = Some(pair.as_str().to_owned()),
                    _ => {
                        // Ignore other rules
                    }
                }
            }

            match (address, name) {
                (Some(address), Some(name)) => {
                    state.symbols.push(Symbol {
                        location: address,
                        name,
                    });
                    Ok(())
                }
                _ => Err(pest_error(span, format!("Missing address and name")).into()),
            }
        }
        _ => Err(pest_error(
            span,
            format!("Unknown rule {:?} in mmap_section", pair.as_rule()),
        )
        .into()),
    }
}

fn process_linker_script_map(state: &mut MapFile, contents: Pairs<Rule>) -> Result<()> {
    for pair in contents {
        match pair.as_rule() {
            Rule::linker_directive => {}
            Rule::mmap_section => {
                process_mmap_section(state, pair).context("read mmap section")?;
            }
            Rule::blank_line => {}
            _ => {
                return Err(anyhow!(
                    "Unknown rule for linker_script_map {:?}",
                    pair.as_rule()
                ))
            }
        }
    }

    Ok(())
}

pub(crate) fn parse_map_file(file_contents: &str) -> Result<MapFile> {
    let mut contents = MapParser::parse(Rule::file, file_contents).context("Run PEST parser")?;

    let mut state = MapFile {
        symbols: Vec::new(),
        padding: 0,
    };

    let contents = contents.next().ok_or_else(|| anyhow!("No file?"))?;
    assert!(contents.as_rule() == Rule::file);
    for pair in contents.into_inner() {
        match pair.as_rule() {
            Rule::archive_members => {}
            Rule::discarded_input_sections => {}
            Rule::memory_configuration => {}
            Rule::linker_script_map => {
                process_linker_script_map(&mut state, pair.into_inner())?;
            }
            Rule::cross_reference_table => {}
            _ => return Err(anyhow!("Unknown rule for file {:?}", pair.as_rule())),
        }
    }

    Ok(state)
}
