//! Emits this crate's observed cross-validator conformance report.

use std::{
    collections::BTreeMap,
    fs,
    io::Write as _,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use openakb_validate::{Mode, extract_citations, validate};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
struct Report {
    implementation: &'static str,
    version: &'static str,
    spec_major: u8,
    fixtures: BTreeMap<String, FixtureReport>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum FixtureReport {
    Descriptor {
        lenient: Vec<String>,
        strict: Vec<String>,
    },
    Content {
        citations: Vec<Vec<String>>,
    },
}

fn main() -> Result<()> {
    let conformance_root = repo_root().join("conformance");
    let mut fixtures = BTreeMap::new();

    collect_descriptor_fixtures(&conformance_root, "valid", &mut fixtures)?;
    collect_descriptor_fixtures(&conformance_root, "invalid", &mut fixtures)?;
    collect_descriptor_fixtures(&conformance_root, "forward-compat", &mut fixtures)?;
    collect_content_fixtures(&conformance_root, &mut fixtures)?;

    let report = Report {
        implementation: "openakb-validate-rust",
        version: env!("CARGO_PKG_VERSION"),
        spec_major: 1,
        fixtures,
    };

    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    serde_json::to_writer_pretty(&mut handle, &report).context("write report JSON")?;
    handle.write_all(b"\n").context("write trailing newline")?;
    Ok(())
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn collect_descriptor_fixtures(
    conformance_root: &Path,
    kind: &str,
    fixtures: &mut BTreeMap<String, FixtureReport>,
) -> Result<()> {
    for case_dir in case_dirs(conformance_root, kind)? {
        let case_name = case_dir_name(&case_dir)?;
        let descriptor = read_json(&case_dir.join("openakb.json"))?;
        fixtures.insert(
            format!("{kind}/{case_name}"),
            FixtureReport::Descriptor {
                lenient: codes(&descriptor, Mode::Lenient),
                strict: codes(&descriptor, Mode::Strict),
            },
        );
    }
    Ok(())
}

fn collect_content_fixtures(
    conformance_root: &Path,
    fixtures: &mut BTreeMap<String, FixtureReport>,
) -> Result<()> {
    for case_dir in case_dirs(conformance_root, "content")? {
        let case_name = case_dir_name(&case_dir)?;
        let markdown = fs::read_to_string(case_dir.join("content.md"))
            .with_context(|| format!("read content fixture {case_name}"))?;
        let citations = extract_citations(&markdown)
            .into_iter()
            .map(|citation| citation.ids)
            .collect();
        fixtures.insert(
            format!("content/{case_name}"),
            FixtureReport::Content { citations },
        );
    }
    Ok(())
}

fn case_dirs(conformance_root: &Path, kind: &str) -> Result<Vec<PathBuf>> {
    let dir = conformance_root.join(kind);
    let mut paths = fs::read_dir(&dir)
        .with_context(|| format!("read fixture directory {}", dir.display()))?
        .map(|entry| {
            let entry = entry.with_context(|| format!("read entry in {}", dir.display()))?;
            Ok(entry.path())
        })
        .collect::<Result<Vec<_>>>()?;
    paths.retain(|path| path.is_dir());
    paths.sort();
    Ok(paths)
}

fn case_dir_name(case_dir: &Path) -> Result<String> {
    Ok(case_dir
        .file_name()
        .context("fixture directory has no final path component")?
        .to_string_lossy()
        .into_owned())
}

fn read_json(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn codes(descriptor: &Value, mode: Mode) -> Vec<String> {
    validate(descriptor, mode)
        .codes()
        .into_iter()
        .map(|code| code.as_str().to_owned())
        .collect()
}
