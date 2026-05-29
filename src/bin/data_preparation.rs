use anyhow::{Context, Result};
use clap::Parser;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(about = "Parse crates.io-index and export a core dependency subgraph.")]
struct Args {
    #[arg(long, value_name = "DIR")]
    index: PathBuf,
    #[arg(long, value_name = "DIR", default_value = "outputs")]
    output: PathBuf,
    #[arg(long, value_name = "FILE")]
    dump: Option<PathBuf>,
    #[arg(long, default_value_t = 100000)]
    top_n: usize,
    #[arg(long, default_value_t = false)]
    include_optional: bool,
    #[arg(long, default_value_t = false)]
    include_dev: bool,
    #[arg(long, default_value_t = false)]
    include_build: bool,
}

#[derive(Deserialize)]
struct IndexRecord {
    name: String,
    vers: String,
    yanked: bool,
    deps: Vec<IndexDep>,
}

#[derive(Deserialize)]
struct IndexDep {
    name: String,
    optional: bool,
    kind: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
struct CrateData {
    name: String,
    deps: Vec<String>,
}

#[derive(Serialize, Deserialize, Default, Clone)]
struct ErrorCounts {
    io_errors: usize,
    json_errors: usize,
    version_errors: usize,
}

#[derive(Serialize)]
struct Filters {
    include_optional: bool,
    include_dev: bool,
    include_build: bool,
    top_n: usize,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
struct DumpFilters {
    include_optional: bool,
    include_dev: bool,
    include_build: bool,
}

#[derive(Serialize, Deserialize)]
struct CrateDump {
    index_path: String,
    filters: DumpFilters,
    files_scanned: usize,
    skipped_yanked_only: usize,
    errors: ErrorCounts,
    crates: Vec<CrateData>,
}

#[derive(Serialize)]
struct Summary {
    index_path: String,
    output_dir: String,
    files_scanned: usize,
    total_crates: usize,
    total_edges: usize,
    skipped_yanked_only: usize,
    core_nodes: usize,
    core_edges: usize,
    filters: Filters,
    errors: ErrorCounts,
}

#[derive(Clone)]
struct NodeStat {
    name: String,
    in_degree: usize,
    out_degree: usize,
}

pub fn main() -> Result<()> {
    let args = Args::parse();
    fs::create_dir_all(&args.output).context("create output directory")?;

    let dump_filters = DumpFilters {
        include_optional: args.include_optional,
        include_dev: args.include_dev,
        include_build: args.include_build,
    };
    let dump_path = args
        .dump
        .clone()
        .unwrap_or_else(|| args.output.join("crates_dump.json"));

    let CrateDump {
        mut crates,
        errors,
        files_scanned,
        skipped_yanked_only,
        ..
    } = load_or_build_dump(&args, &dump_filters, &dump_path)?;

    crates.sort_by(|a, b| a.name.cmp(&b.name));

    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut out_degree: HashMap<String, usize> = HashMap::new();
    let mut total_edges = 0usize;

    for data in &crates {
        total_edges += data.deps.len();
        out_degree.insert(data.name.clone(), data.deps.len());
        in_degree.entry(data.name.clone()).or_insert(0);
        for dep in &data.deps {
            *in_degree.entry(dep.clone()).or_insert(0) += 1;
        }
    }

    let mut nodes: Vec<NodeStat> = crates
        .iter()
        .map(|data| NodeStat {
            name: data.name.clone(),
            in_degree: *in_degree.get(&data.name).unwrap_or(&0),
            out_degree: *out_degree.get(&data.name).unwrap_or(&0),
        })
        .collect();

    nodes.sort_by(|a, b| {
        b.in_degree
            .cmp(&a.in_degree)
            .then_with(|| a.name.cmp(&b.name))
    });

    let core_count = args.top_n.min(nodes.len());
    let core_set: HashSet<String> = nodes
        .iter()
        .take(core_count)
        .map(|node| node.name.clone())
        .collect();

    let core_nodes: Vec<NodeStat> = nodes
        .iter()
        .filter(|node| core_set.contains(&node.name))
        .cloned()
        .collect();

    let mut core_edges: Vec<(String, String)> = Vec::new();
    for data in &crates {
        if !core_set.contains(&data.name) {
            continue;
        }
        for dep in &data.deps {
            if core_set.contains(dep) {
                core_edges.push((data.name.clone(), dep.clone()));
            }
        }
    }

    core_edges.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

    let nodes_path = args.output.join("core_nodes.csv");
    let edges_path = args.output.join("core_edges.csv");
    write_nodes_csv(&nodes_path, &core_nodes)?;
    write_edges_csv(&edges_path, &core_edges)?;

    let summary = Summary {
        index_path: args.index.display().to_string(),
        output_dir: args.output.display().to_string(),
        files_scanned,
        total_crates: crates.len(),
        total_edges,
        skipped_yanked_only,
        core_nodes: core_nodes.len(),
        core_edges: core_edges.len(),
        filters: Filters {
            include_optional: args.include_optional,
            include_dev: args.include_dev,
            include_build: args.include_build,
            top_n: args.top_n,
        },
        errors,
    };

    let summary_path = args.output.join("summary.json");
    let summary_json = serde_json::to_string_pretty(&summary)?;
    fs::write(summary_path, summary_json)?;

    println!("Crates: {}", summary.total_crates);
    println!("Edges (filtered): {}", summary.total_edges);
    println!("Core nodes: {}", summary.core_nodes);
    println!("Core edges: {}", summary.core_edges);
    println!("Output: {}", args.output.display());

    Ok(())
}

fn load_or_build_dump(
    args: &Args,
    dump_filters: &DumpFilters,
    dump_path: &Path,
) -> Result<CrateDump> {
    if dump_path.exists() {
        match load_crate_dump(dump_path, &args.index, dump_filters) {
            Ok(Some(dump)) => {
                println!(
                    "Loaded crates dump: {} ({} crates)",
                    dump_path.display(),
                    dump.crates.len()
                );
                return Ok(dump);
            }
            Ok(None) => {
                println!("Crates dump does not match current filters/index. Rebuilding...");
            }
            Err(err) => {
                println!("Failed to read crates dump: {err}. Rebuilding...");
            }
        }
    }

    let dump = build_crate_dump(args, dump_filters)?;
    if let Some(parent) = dump_path.parent() {
        fs::create_dir_all(parent).context("create dump directory")?;
    }
    if let Err(err) = write_crate_dump(dump_path, &dump) {
        eprintln!("Warning: failed to write crates dump: {err}");
    }
    Ok(dump)
}

fn load_crate_dump(
    path: &Path,
    index_path: &Path,
    dump_filters: &DumpFilters,
) -> Result<Option<CrateDump>> {
    let file = File::open(path).with_context(|| format!("open dump {}", path.display()))?;
    let reader = BufReader::new(file);
    let dump: CrateDump =
        serde_json::from_reader(reader).with_context(|| format!("read dump {}", path.display()))?;

    if dump.index_path != index_path.display().to_string() {
        return Ok(None);
    }
    if dump.filters != *dump_filters {
        return Ok(None);
    }

    Ok(Some(dump))
}

fn write_crate_dump(path: &Path, dump: &CrateDump) -> Result<()> {
    let file = File::create(path).with_context(|| format!("create dump {}", path.display()))?;
    let writer = BufWriter::new(file);
    serde_json::to_writer(writer, dump).context("write dump")?;
    Ok(())
}

fn build_crate_dump(args: &Args, dump_filters: &DumpFilters) -> Result<CrateDump> {
    let mut crates: Vec<CrateData> = Vec::new();
    let mut errors = ErrorCounts::default();
    let mut files_scanned = 0usize;
    let mut skipped_yanked_only = 0usize;

    let walker = WalkDir::new(&args.index)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| entry.file_name() != ".git");

    for entry in walker {
        let entry = match entry {
            Ok(value) => value,
            Err(_) => {
                errors.io_errors += 1;
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }
        if !is_index_file(entry.path()) {
            continue;
        }

        files_scanned += 1;
        match parse_crate_file(entry.path(), args, &mut errors) {
            Ok(Some(data)) => crates.push(data),
            Ok(None) => skipped_yanked_only += 1,
            Err(_) => errors.io_errors += 1,
        }
    }

    Ok(CrateDump {
        index_path: args.index.display().to_string(),
        filters: dump_filters.clone(),
        files_scanned,
        skipped_yanked_only,
        errors,
        crates,
    })
}

fn is_index_file(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|name| name.to_str()) {
        Some(name) => name,
        None => return false,
    };
    if file_name == "config.json" || file_name == "README.md" {
        return false;
    }
    true
}

fn parse_crate_file(
    path: &Path,
    args: &Args,
    errors: &mut ErrorCounts,
) -> Result<Option<CrateData>> {
    let file = File::open(path).with_context(|| format!("open file {}", path.display()))?;
    let reader = BufReader::new(file);
    let mut best_version: Option<Version> = None;
    let mut best_name: Option<String> = None;
    let mut best_deps: Vec<String> = Vec::new();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let record: IndexRecord = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => {
                errors.json_errors += 1;
                continue;
            }
        };
        if record.yanked {
            continue;
        }
        let version = match Version::parse(&record.vers) {
            Ok(value) => value,
            Err(_) => {
                errors.version_errors += 1;
                continue;
            }
        };
        let replace = match &best_version {
            None => true,
            Some(current) => version > *current,
        };
        if replace {
            let deps = filter_deps(&record.deps, args);
            best_version = Some(version);
            best_name = Some(record.name);
            best_deps = deps;
        }
    }

    if let Some(name) = best_name {
        Ok(Some(CrateData {
            name,
            deps: best_deps,
        }))
    } else {
        Ok(None)
    }
}

fn filter_deps(deps: &[IndexDep], args: &Args) -> Vec<String> {
    let mut unique = HashSet::new();
    for dep in deps {
        if dep.optional && !args.include_optional {
            continue;
        }
        let kind = dep.kind.as_deref().unwrap_or("normal");
        let include = match kind {
            "normal" => true,
            "dev" => args.include_dev,
            "build" => args.include_build,
            _ => true,
        };
        if !include {
            continue;
        }
        unique.insert(dep.name.clone());
    }

    let mut deps: Vec<String> = unique.into_iter().collect();
    deps.sort();
    deps
}

fn write_nodes_csv(path: &Path, nodes: &[NodeStat]) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "name,in_degree,out_degree")?;
    for node in nodes {
        writeln!(
            writer,
            "{},{},{}",
            node.name, node.in_degree, node.out_degree
        )?;
    }
    Ok(())
}

fn write_edges_csv(path: &Path, edges: &[(String, String)]) -> Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    writeln!(writer, "src,dst")?;
    for (src, dst) in edges {
        writeln!(writer, "{},{}", src, dst)?;
    }
    Ok(())
}
