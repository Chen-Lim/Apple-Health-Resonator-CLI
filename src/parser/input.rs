use std::fs::File;
use std::io::BufReader;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use zip::ZipArchive;

use crate::parser::file_names::is_supported_export_xml_name;

pub(crate) enum InputSource {
    Xml(BufReader<File>),
    Zip {
        archive: ZipArchive<File>,
        entry_index: usize,
    },
}

pub(crate) fn open_input(path: &Path) -> Result<InputSource> {
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
    let entry_index = archive
        .file_names()
        .enumerate()
        .find_map(|(idx, name)| is_supported_export_xml_name(name).then_some(idx))
        .ok_or_else(|| anyhow!("zip archive does not contain a supported Apple Health export xml file"))?;

    Ok(InputSource::Zip {
        archive,
        entry_index,
    })
}
