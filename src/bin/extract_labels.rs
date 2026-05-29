use anyhow::{Context, Result};
use clap::Parser;
use csv::StringRecord;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(about = "Extract crate categories/keywords from a crates.io db dump.")]
struct Args {
    #[arg(long, value_name = "DIR")]
    db_dir: PathBuf,
    #[arg(long, value_name = "FILE", default_value = "outputs/analysis/crate_labels.csv")]
    out: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let data_dir = args.db_dir.join("data");
    let crates_path = data_dir.join("crates.csv");
    let categories_path = data_dir.join("categories.csv");
    let crates_categories_path = data_dir.join("crates_categories.csv");
    let keywords_path = data_dir.join("keywords.csv");
    let crates_keywords_path = data_dir.join("crates_keywords.csv");

    let crate_names = read_id_name(&crates_path, "id", "name")
        .with_context(|| format!("read crates from {}", crates_path.display()))?;
    let category_names = read_id_name(&categories_path, "id", "category")
        .with_context(|| format!("read categories from {}", categories_path.display()))?;
    let keyword_names = read_id_name(&keywords_path, "id", "keyword")
        .with_context(|| format!("read keywords from {}", keywords_path.display()))?;

    let crate_categories = read_relation(&crates_categories_path, "crate_id", "category_id")
        .with_context(|| format!("read crates_categories from {}", crates_categories_path.display()))?;
    let crate_keywords = read_relation(&crates_keywords_path, "crate_id", "keyword_id")
        .with_context(|| format!("read crates_keywords from {}", crates_keywords_path.display()))?;

    let mut category_map: HashMap<i64, HashSet<String>> = HashMap::new();
    for (crate_id, category_id) in crate_categories {
        if let Some(name) = category_names.get(&category_id) {
            category_map
                .entry(crate_id)
                .or_default()
                .insert(name.clone());
        }
    }

    let mut keyword_map: HashMap<i64, HashSet<String>> = HashMap::new();
    for (crate_id, keyword_id) in crate_keywords {
        if let Some(name) = keyword_names.get(&keyword_id) {
            keyword_map
                .entry(crate_id)
                .or_default()
                .insert(name.clone());
        }
    }

    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent).context("create output directory")?;
    }

    let mut writer = csv::Writer::from_path(&args.out)?;
    writer.write_record(["crate", "categories", "keywords"])?;

    let mut crate_ids: Vec<i64> = crate_names.keys().copied().collect();
    crate_ids.sort_unstable();

    for crate_id in crate_ids {
        let name = match crate_names.get(&crate_id) {
            Some(value) => value,
            None => continue,
        };
        let mut categories: Vec<String> = category_map
            .get(&crate_id)
            .map(|values| values.iter().cloned().collect())
            .unwrap_or_default();
        let mut keywords: Vec<String> = keyword_map
            .get(&crate_id)
            .map(|values| values.iter().cloned().collect())
            .unwrap_or_default();
        categories.sort();
        keywords.sort();

        writer.write_record([
            name.as_str(),
            &categories.join(";"),
            &keywords.join(";"),
        ])?;
    }

    writer.flush()?;

    println!("Output: {}", args.out.display());
    println!("Crates: {}", crate_names.len());
    println!("Crates with categories: {}", category_map.len());
    println!("Crates with keywords: {}", keyword_map.len());

    Ok(())
}

fn read_id_name(path: &Path, id_col: &str, name_col: &str) -> Result<HashMap<i64, String>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    let id_idx = header_index(&headers, id_col)
        .with_context(|| format!("missing column {}", id_col))?;
    let name_idx = header_index(&headers, name_col)
        .with_context(|| format!("missing column {}", name_col))?;

    let mut map = HashMap::new();
    for record in reader.records() {
        let record = record?;
        let id = record
            .get(id_idx)
            .and_then(|value| value.parse::<i64>().ok());
        let name = record.get(name_idx);
        if let (Some(id), Some(name)) = (id, name) {
            map.insert(id, name.to_string());
        }
    }
    Ok(map)
}

fn read_relation(path: &Path, left_col: &str, right_col: &str) -> Result<Vec<(i64, i64)>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    let left_idx = header_index(&headers, left_col)
        .with_context(|| format!("missing column {}", left_col))?;
    let right_idx = header_index(&headers, right_col)
        .with_context(|| format!("missing column {}", right_col))?;

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        let left = record
            .get(left_idx)
            .and_then(|value| value.parse::<i64>().ok());
        let right = record
            .get(right_idx)
            .and_then(|value| value.parse::<i64>().ok());
        if let (Some(left), Some(right)) = (left, right) {
            rows.push((left, right));
        }
    }

    Ok(rows)
}

fn header_index(headers: &StringRecord, name: &str) -> Option<usize> {
    headers.iter().position(|value| value == name)
}
