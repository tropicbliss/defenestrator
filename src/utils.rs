use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use std::{fs::read_to_string, path::PathBuf, string::ToString};

pub fn get_names(path: PathBuf) -> Result<Vec<String>> {
    let file = read_to_string(path)?;
    Ok(file.lines().map(ToString::to_string).collect())
}

pub fn get_name_validity(names: &[String]) -> NameValidityData {
    let mut invalid_names = Vec::new();
    let mut valid_names = Vec::new();
    for name in names {
        if is_invalid_predicate(name) {
            invalid_names.push(name.to_string());
        } else {
            valid_names.push(name.to_string());
        }
    }
    NameValidityData {
        valid_names,
        invalid_names,
    }
}

fn is_invalid_predicate(name: &str) -> bool {
    // Alphanumeric + underscore characters allowed
    lazy_static! {
        static ref RE: Regex = Regex::new("[^a-zA-Z0-9_.]").unwrap();
    }
    name.len() < 3 || name.len() > 16 || RE.is_match(name)
}

pub struct NameValidityData {
    pub valid_names: Vec<String>,
    pub invalid_names: Vec<String>,
}
