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
    pub address: u64,
    pub size: u64,
    pub name: String,
}

pub(crate) struct MapFile {
    pub symbols: Vec<Symbol>,
    pub padding: u64,
}

struct MapFileParseState {
    out: MapFile,
    current_symbol: Option<Symbol>,
}

impl MapFile {
    pub(crate) fn compute_symbol_sizes(&mut self) -> HashMap<String, u64> {
        self.symbols
            .sort_unstable_by(|a, b| a.address.cmp(&b.address));
        self.symbols
            .windows(2)
            .map(|symbols| {
                let symbol = &symbols[0];
                let next_symbol = &symbols[1];
                let size = next_symbol.address - symbol.address;

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

fn hex_number_to_u64(contents: Pair<Rule>) -> Result<u64> {
    let span = contents.as_span();
    let inner = contents
        .into_inner()
        .next()
        .ok_or_else(|| pest_error(span, "Missing hex_number_digits"))?;
    let span = inner.as_span();

    match inner.as_rule() {
        Rule::hex_number_digits => Ok(u64::from_str_radix(inner.as_str(), 16)?),
        rule => Err(pest_error(span, format!("bad span type {:?}", rule)).into()),
    }
}

fn process_mmap_section_name(contents: Pair<Rule>) -> Result<Option<String>> {
    let span = contents.as_span();
    let pair = contents
        .into_inner()
        .next()
        .ok_or_else(|| pest_error(span, format!("Expected section name")))?;

    match pair.as_rule() {
        Rule::mmap_section_name_blank => Ok(None),
        Rule::section_name => Ok(Some(pair.get_input().to_string())),
        _ => Err(pest_error(
            pair.as_span(),
            format!("Unexpected rule {:?}", pair.as_rule()),
        )
        .into()),
    }
}

fn process_mmap_section(state: &mut MapFileParseState, contents: Pair<Rule>) -> Result<()> {
    let span = contents.as_span();
    let mut section_name = None;
    let mut address = None;
    let mut section_size = None;
    let mut source = None;

    for pair in contents.into_inner() {
        let span = pair.as_span();
        // mmap_section_address ~ mmap_section_size ~ (mmap_source)? ~ "\n"
        match pair.as_rule() {
            Rule::mmap_section_name => {
                section_name = process_mmap_section_name(pair)?;
            }
            Rule::mmap_section_address => {
                address =
                    Some(
                        hex_number_to_u64(pair.into_inner().next().ok_or_else(|| {
                            pest_error(span, "Expected inner for section address")
                        })?)
                        .context("mmap_section_address extraction")?,
                    )
            }
            Rule::mmap_section_size => {
                section_size = match pair.into_inner().next() {
                    Some(pair) => {
                        Some(hex_number_to_u64(pair).context("mmap_section_size extraction")?)
                    }
                    None => None,
                }
            }
            Rule::mmap_source => source = Some(pair.as_str()),
            _ => {}
        }
    }

    let address = if let Some(address) = address {
        address
    } else {
        // [!provide] doesn't set an address
        return Ok(());
    };

    if let Some(section_name) = section_name {
        let section_size =
            section_size.ok_or_else(|| pest_error(span, "Named sections must have a size"))?;
        if section_name.contains("*fill*") {
            state.out.padding += section_size;
        }
    }

    if let Some(mut c) = state.current_symbol.take() {
        if address < c.address {
            return Err(pest_error(span, "Out of order symbol").into());
        }
        c.size = address - c.address;
        state.out.symbols.push(c);
    }

    if let Some(source) = source {
        if source.starts_with("        ") {
            // This is a linker directive
            return Ok(());
        }

        if source.contains("/") {
            // This is a file path
            return Ok(());
        }

        state.current_symbol = Some(Symbol {
            address,
            size: 0,
            name: source.to_string(),
        })
    }

    Ok(())
}

fn process_linker_script_map(state: &mut MapFileParseState, contents: Pairs<Rule>) -> Result<()> {
    let mut discarding = false;
    for pair in contents {
        match pair.as_rule() {
            Rule::linker_directive => {
                let pair = pair.into_inner().next();
                if let Some(pair) = pair {
                    match pair.as_rule() {
                        Rule::linker_directive_discard => {
                            discarding = true;
                        }
                        _ => {}
                    }
                }
            }
            Rule::mmap_section => {
                if !discarding {
                    process_mmap_section(state, pair).context("read mmap section")?;
                }
            }
            Rule::mmap_section_glob => {
                // nothing to do with this
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

    let mut state = MapFileParseState {
        out: MapFile {
            symbols: Vec::new(),
            padding: 0,
        },
        current_symbol: None,
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

    Ok(state.out)
}
