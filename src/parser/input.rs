use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use zip::ZipArchive;

use crate::parser::file_names::{is_simple_health_export_csv_name, is_supported_export_xml_name};

pub(crate) enum InputSource {
    Xml(BufReader<File>),
    Zip {
        archive: ZipArchive<File>,
        entry_index: usize,
    },
    /// SimpleHealthExportCSV bundle: a zip whose entries are per-type CSV files.
    CsvZip {
        archive: ZipArchive<File>,
        entry_indices: Vec<usize>,
    },
    /// Already-unpacked SimpleHealthExportCSV directory.
    CsvDir {
        csv_files: Vec<PathBuf>,
    },
}

pub(crate) fn open_input(path: &Path) -> Result<InputSource> {
    if path.is_dir() {
        return open_csv_dir(path);
    }
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("xml") => open_xml(path),
        Some("zip") => open_zip(path),
        _ => Err(anyhow!("unsupported input format: {}", path.display())),
    }
}

fn open_xml(path: &Path) -> Result<InputSource> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    Ok(InputSource::Xml(BufReader::new(file)))
}

fn open_zip(path: &Path) -> Result<InputSource> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let archive = ZipArchive::new(file).context("failed to read zip archive")?;
    classify_zip(archive, path)
}

fn classify_zip(mut archive: ZipArchive<File>, path: &Path) -> Result<InputSource> {
    let mut xml_idx: Option<usize> = None;
    let mut csv_indices: Vec<usize> = Vec::new();
    for idx in 0..archive.len() {
        let entry = archive.by_index_raw(idx)?;
        // The UTF-8 flag may not be set even when bytes are valid UTF-8 (common
        // in Apple Health exports). Decode raw bytes as UTF-8 first, falling
        // back to the crate's default CP437 decoding.
        let name_owned: String = match std::str::from_utf8(entry.name_raw()) {
            Ok(s) => s.to_string(),
            Err(_) => entry.name().to_string(),
        };
        if xml_idx.is_none() && is_supported_export_xml_name(&name_owned) {
            xml_idx = Some(idx);
        } else if is_simple_health_export_csv_name(&name_owned) {
            csv_indices.push(idx);
        }
    }

    if let Some(entry_index) = xml_idx {
        return Ok(InputSource::Zip {
            archive,
            entry_index,
        });
    }
    if !csv_indices.is_empty() {
        return Ok(InputSource::CsvZip {
            archive,
            entry_indices: csv_indices,
        });
    }
    Err(anyhow!(
        "zip archive {} contains neither an Apple Health export xml nor SimpleHealthExportCSV files",
        path.display()
    ))
}

fn open_csv_dir(dir: &Path) -> Result<InputSource> {
    let mut csv_files = Vec::new();
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
    {
        let entry = entry?;
        let p = entry.path();
        if !p.is_file() {
            continue;
        }
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if is_simple_health_export_csv_name(name) {
                csv_files.push(p);
            }
        }
    }
    if csv_files.is_empty() {
        return Err(anyhow!(
            "directory {} contains no SimpleHealthExportCSV files",
            dir.display()
        ));
    }
    csv_files.sort();
    Ok(InputSource::CsvDir { csv_files })
}
