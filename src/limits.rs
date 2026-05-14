use anyhow::{Context, Result, bail};
use std::io::{Cursor, Read, Seek};
use zip::read::ZipArchive;
use zip::result::ZipError;

pub(crate) const DEFAULT_MAX_ZIP_ENTRIES: usize = 10_000;
pub(crate) const DEFAULT_MAX_ZIP_ENTRY_BYTES: usize = 50 * 1024 * 1024;
pub(crate) const DEFAULT_MAX_ZIP_COMPRESSION_RATIO: u64 = 200;

#[derive(Debug, Clone, Copy)]
pub(crate) struct Limits {
    pub max_zip_entries: usize,
    pub max_zip_entry_bytes: usize,
    pub max_zip_compression_ratio: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_zip_entries: DEFAULT_MAX_ZIP_ENTRIES,
            max_zip_entry_bytes: DEFAULT_MAX_ZIP_ENTRY_BYTES,
            max_zip_compression_ratio: DEFAULT_MAX_ZIP_COMPRESSION_RATIO,
        }
    }
}

pub(crate) type ZipReader<'a> = ZipArchive<Cursor<&'a [u8]>>;

pub(crate) fn open_zip_archive<'a>(bytes: &'a [u8], label: &str) -> Result<ZipReader<'a>> {
    let mut zip = ZipArchive::new(Cursor::new(bytes))
        .with_context(|| format!("failed to open {label} archive"))?;
    validate_zip_archive(&mut zip, Limits::default())
        .with_context(|| format!("{label} archive failed safety checks"))?;
    Ok(zip)
}

pub(crate) fn read_zip_text<R: Read + Seek>(zip: &mut ZipArchive<R>, name: &str) -> Result<String> {
    let bytes = read_zip_bytes(zip, name)?;
    String::from_utf8(bytes).with_context(|| format!("zip entry is not UTF-8 text: {name}"))
}

pub(crate) fn read_zip_text_optional<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    name: &str,
) -> Result<Option<String>> {
    read_zip_bytes_optional(zip, name)?
        .map(|bytes| {
            String::from_utf8(bytes).with_context(|| format!("zip entry is not UTF-8 text: {name}"))
        })
        .transpose()
}

pub(crate) fn read_zip_bytes<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    name: &str,
) -> Result<Vec<u8>> {
    read_zip_bytes_optional(zip, name)?.with_context(|| format!("zip entry not found: {name}"))
}

pub(crate) fn read_zip_bytes_optional<R: Read + Seek>(
    zip: &mut ZipArchive<R>,
    name: &str,
) -> Result<Option<Vec<u8>>> {
    let file = match zip.by_name(name) {
        Ok(file) => file,
        Err(ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("failed to open zip entry: {name}")),
    };

    read_limited_zip_file(file, Limits::default())
        .map(Some)
        .with_context(|| format!("failed to read zip entry: {name}"))
}

fn validate_zip_archive<R: Read + Seek>(zip: &mut ZipArchive<R>, limits: Limits) -> Result<()> {
    if zip.len() > limits.max_zip_entries {
        bail!(
            "zip entry count {} exceeds limit {}",
            zip.len(),
            limits.max_zip_entries
        );
    }

    for idx in 0..zip.len() {
        let file = zip
            .by_index(idx)
            .with_context(|| format!("failed to inspect zip entry #{idx}"))?;
        validate_zip_entry(file.name(), file.compressed_size(), file.size(), limits)?;
    }

    Ok(())
}

fn read_limited_zip_file(file: zip::read::ZipFile<'_>, limits: Limits) -> Result<Vec<u8>> {
    validate_zip_entry(file.name(), file.compressed_size(), file.size(), limits)?;

    let mut bytes = Vec::new();
    let mut limited = file.take(limits.max_zip_entry_bytes as u64 + 1);
    limited.read_to_end(&mut bytes)?;
    if bytes.len() > limits.max_zip_entry_bytes {
        bail!(
            "zip entry exceeds decompressed size limit of {} bytes",
            limits.max_zip_entry_bytes
        );
    }
    Ok(bytes)
}

fn validate_zip_entry(
    name: &str,
    compressed_size: u64,
    uncompressed_size: u64,
    limits: Limits,
) -> Result<()> {
    if uncompressed_size > limits.max_zip_entry_bytes as u64 {
        bail!(
            "zip entry {name} size {uncompressed_size} exceeds limit {}",
            limits.max_zip_entry_bytes
        );
    }

    if compressed_size == 0 {
        if uncompressed_size > 0 {
            bail!("zip entry {name} has non-empty content with zero compressed size");
        }
        return Ok(());
    }

    let ratio = uncompressed_size / compressed_size;
    if ratio > limits.max_zip_compression_ratio {
        bail!(
            "zip entry {name} compression ratio {ratio} exceeds limit {}",
            limits.max_zip_compression_ratio
        );
    }

    Ok(())
}
