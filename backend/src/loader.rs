use std::{collections::HashSet, fs, path::Path};

use anyhow::{bail, Context, Result};

use crate::city::City;

pub fn load_cities() -> Result<Vec<City>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("cities.json");

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed reading city data at {}", path.display()))?;
    let cities: Vec<City> = serde_json::from_str(&raw)
        .with_context(|| format!("failed parsing city JSON at {}", path.display()))?;

    if cities.is_empty() {
        bail!("city dataset is empty")
    }

    Ok(cities)
}

pub fn load_allowed_words() -> Result<HashSet<String>> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("allowed_words.txt");

    let raw = fs::read_to_string(&path)
        .with_context(|| format!("failed reading dictionary at {}", path.display()))?;

    let words = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(normalize_word)
        .filter(|word| word.len() >= 3)
        .collect::<HashSet<_>>();

    if words.is_empty() {
        bail!("dictionary is empty")
    }

    Ok(words)
}

fn normalize_word(input: &str) -> String {
    input
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.to_ascii_uppercase())
        .collect()
}
