use anyhow::Result;
use console::style;
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    fs::read_to_string,
    io::{StdoutLock, Write},
    path::Path,
    string::ToString,
};

pub fn get_names(path: &Path) -> Result<Vec<String>> {
    let file = read_to_string(path)?;
    Ok(file
        .lines()
        .map(str::trim)
        .map(ToString::to_string)
        .collect())
}

pub fn get_name_validity(handle: &mut StdoutLock, names: Vec<String>) -> Result<NameValidityData> {
    let mut invalid_names = Vec::new();
    let mut valid_names = Vec::new();
    for name in names {
        if is_invalid_predicate(&name) {
            writeln!(handle, "{} is an invalid name", style(&name).yellow())?;
            invalid_names.push(name);
        } else {
            valid_names.push(name);
        }
    }

    Ok(NameValidityData {
        valid_names,
        invalid_names,
    })
}

fn is_invalid_predicate(name: &str) -> bool {
    // Only alphanumeric + underscore characters allowed
    lazy_static! {
        static ref RE: Regex = Regex::new("[^a-zA-Z0-9_.]").unwrap();
    }
    name.len() < 3 || name.len() > 16 || RE.is_match(name)
}

pub struct NameValidityData {
    pub valid_names: Vec<String>,
    pub invalid_names: Vec<String>,
}
