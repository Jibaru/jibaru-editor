//! ZIP container I/O. A `.wfp` is a plain ZIP mixing stored and deflated
//! entries. Entries are kept in their original order with their original
//! compression method, timestamp and permissions so that untouched entries
//! are written back byte-for-byte.

use crate::{Error, Result};
use std::io::{Read, Seek, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, DateTime, ZipArchive, ZipWriter};

/// One file inside the archive.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub name: String,
    pub data: Vec<u8>,
    /// True for deflated entries, false for stored ones.
    pub deflated: bool,
    /// MS-DOS timestamp as (year, month, day, hour, minute, second).
    pub modified: Option<(u16, u8, u8, u8, u8, u8)>,
    pub unix_mode: Option<u32>,
}

impl Entry {
    pub fn new(name: impl Into<String>, data: Vec<u8>) -> Self {
        Self { name: name.into(), data, deflated: true, modified: None, unix_mode: None }
    }
}

pub fn read_entries<R: Read + Seek>(reader: R) -> Result<Vec<Entry>> {
    let mut zip = ZipArchive::new(reader)?;
    let mut out = Vec::with_capacity(zip.len());
    for i in 0..zip.len() {
        let mut f = zip.by_index(i)?;
        if f.is_dir() {
            continue;
        }
        let name = f.name()?.to_string();
        let mut data = Vec::with_capacity(f.size() as usize);
        f.read_to_end(&mut data)?;
        let modified = f.last_modified().map(|d| (d.year(), d.month(), d.day(), d.hour(), d.minute(), d.second()));
        out.push(Entry {
            name,
            data,
            deflated: f.compression() != CompressionMethod::Stored,
            modified,
            unix_mode: f.unix_mode(),
        });
    }
    Ok(out)
}

pub fn write_entries<W: Write + Seek>(writer: W, entries: &[Entry]) -> Result<()> {
    let mut zip = ZipWriter::new(writer);
    for e in entries {
        let method = if e.deflated { CompressionMethod::Deflated } else { CompressionMethod::Stored };
        let mut opts =
            SimpleFileOptions::default().compression_method(method).large_file(e.data.len() > u32::MAX as usize);
        if let Some((y, mo, d, h, mi, s)) = e.modified {
            if let Ok(dt) = DateTime::from_date_and_time(y, mo, d, h, mi, s) {
                opts = opts.last_modified_time(dt);
            }
        }
        if let Some(mode) = e.unix_mode {
            opts = opts.unix_permissions(mode);
        }
        zip.start_file(e.name.as_str(), opts)?;
        zip.write_all(&e.data).map_err(Error::Io)?;
    }
    zip.finish()?;
    Ok(())
}
