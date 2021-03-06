use ansi_term::Colour::Yellow;
use anyhow::Result;
use std::{
    fs::read_to_string,
    io::{stdout, Write},
    path::Path,
    string::ToString,
};

pub fn get_names(path: &Path) -> Result<Vec<String>> {
    let file = read_to_string(path)?;
    let mut result: Vec<_> = file
        .lines()
        .map(str::trim)
        .map(ToString::to_string)
        .collect();
    true_dedup(&mut result);
    Ok(result)
}

pub fn get_name_validity(names: Vec<String>) -> Result<NameValidityData> {
    let mut invalid_names = Vec::new();
    let mut valid_names = Vec::new();
    for name in names {
        if is_valid_predicate(&name) {
            valid_names.push(name);
        } else {
            writeln!(stdout(), "{} is an invalid name", Yellow.paint(&name))?;
            invalid_names.push(name);
        }
    }
    Ok(NameValidityData {
        valid_names,
        invalid_names,
    })
}

fn is_valid_predicate(name: &str) -> bool {
    // Only alphanumeric + underscore characters allowed
    name.len() >= 3 && name.len() <= 16 && name.chars().all(|c| c.is_alphanumeric() || c == '_')
}

pub struct NameValidityData {
    pub valid_names: Vec<String>,
    pub invalid_names: Vec<String>,
}

pub fn true_dedup<T>(vec: &mut Vec<T>)
where
    T: Ord,
{
    vec.sort_unstable();
    vec.dedup();
}

pub fn to_title(s: &str) -> String {
    let mut c = s.chars();
    let mut word = c.next().unwrap().to_ascii_uppercase().to_string();
    word.push_str(&c.as_str().to_ascii_lowercase());
    word
}
