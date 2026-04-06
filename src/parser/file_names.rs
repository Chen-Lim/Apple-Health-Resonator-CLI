use std::path::Path;

const SUPPORTED_EXPORT_XML_NAMES: &[&str] = &["export.xml", "导出.xml"];

pub(crate) fn is_supported_export_xml_name(path: &str) -> bool {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| SUPPORTED_EXPORT_XML_NAMES.contains(&name))
}
