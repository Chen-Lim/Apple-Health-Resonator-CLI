use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use tempfile::TempDir;
use zip::write::SimpleFileOptions;

pub fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

pub fn temp_db(temp_dir: &TempDir) -> PathBuf {
    temp_dir.path().join("health.db")
}

#[allow(dead_code)]
pub fn create_zip_fixture(source_xml: &Path, output_zip: &Path) -> Result<()> {
    create_zip_fixture_with_entry_name(source_xml, output_zip, "export.xml")
}

#[allow(dead_code)]
pub fn create_zip_fixture_with_entry_name(
    source_xml: &Path,
    output_zip: &Path,
    entry_name: &str,
) -> Result<()> {
    let mut xml = String::new();
    File::open(source_xml)?.read_to_string(&mut xml)?;

    let file = File::create(output_zip)?;
    let mut writer = zip::ZipWriter::new(file);
    writer.start_file(entry_name, SimpleFileOptions::default())?;
    writer.write_all(xml.as_bytes())?;
    writer.finish()?;
    Ok(())
}
