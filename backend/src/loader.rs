use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::city::City;

const BASE_CITY_FILE: &str = "cities.json";
const DROPIN_CITY_DIR: &str = "cities";

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CityPayload {
    Single(City),
    Multiple(Vec<City>),
}

pub fn load_cities() -> Result<Vec<City>> {
    let data_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let mut city_files = Vec::new();

    let base_file = data_dir.join(BASE_CITY_FILE);
    if base_file.exists() {
        city_files.push(base_file);
    }

    city_files.extend(list_json_files(&data_dir, Some(BASE_CITY_FILE))?);
    city_files.extend(list_json_files(&data_dir.join(DROPIN_CITY_DIR), None)?);

    if city_files.is_empty() {
        bail!("no city JSON files found under {}", data_dir.display())
    }

    let mut cities = Vec::new();
    let mut index_by_id: HashMap<String, usize> = HashMap::new();

    for path in city_files {
        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed reading city data at {}", path.display()))?;
        let payload: CityPayload = serde_json::from_str(&raw)
            .with_context(|| format!("failed parsing city JSON at {}", path.display()))?;

        for city in payload.into_vec() {
            let city_id = city.id.trim().to_string();
            if city_id.is_empty() {
                bail!("city id is empty in {}", path.display());
            }

            if let Some(existing_index) = index_by_id.get(&city_id).copied() {
                cities[existing_index] = city;
            } else {
                index_by_id.insert(city_id, cities.len());
                cities.push(city);
            }
        }
    }

    if cities.is_empty() {
        bail!("city dataset is empty")
    }

    Ok(cities)
}

impl CityPayload {
    fn into_vec(self) -> Vec<City> {
        match self {
            Self::Single(city) => vec![city],
            Self::Multiple(cities) => cities,
        }
    }
}

fn list_json_files(dir: &Path, exclude_name: Option<&str>) -> Result<Vec<PathBuf>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    if !dir.is_dir() {
        bail!("expected city data directory at {}", dir.display());
    }

    let mut files = Vec::new();
    for entry in
        fs::read_dir(dir).with_context(|| format!("failed reading dir {}", dir.display()))?
    {
        let entry = entry.with_context(|| format!("failed reading entry in {}", dir.display()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }

        if let Some(excluded) = exclude_name {
            if path.file_name().and_then(|value| value.to_str()) == Some(excluded) {
                continue;
            }
        }

        files.push(path);
    }

    files.sort();
    Ok(files)
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
