//! Minimal read-only HFS (Mac OS Standard) reader for floppy disk images.
//!
//! Supports what the S.C.OUT disk needs: the master directory block, the
//! catalog B-tree and files whose forks fit into the three extents stored in
//! the catalog record (no extents overflow file).

use crate::bytes::{mac_roman, slice, u8_at, u16_at, u32_at};
use crate::error::{Error, Result, corrupt};

const MDB_OFFSET: usize = 1024;
const HFS_SIGNATURE: u16 = 0x4244; // "BD"
const FILE_RECORD: u8 = 2;

pub struct Volume<'a> {
    image: &'a [u8],
    block_size: usize,
    first_block: usize,
    catalog: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub file_type: [u8; 4],
    pub creator: [u8; 4],
    data: Fork,
    resource: Fork,
}

#[derive(Debug, Clone, Copy)]
struct Fork {
    len: usize,
    extents: [(u16, u16); 3],
}

impl<'a> Volume<'a> {
    pub fn open(image: &'a [u8]) -> Result<Self> {
        let mdb = MDB_OFFSET;
        if u16_at(image, mdb).ok() != Some(HFS_SIGNATURE) {
            return Err(Error::NotHfs);
        }
        let mut volume = Self {
            image,
            block_size: u32_at(image, mdb + 20)? as usize,
            first_block: u16_at(image, mdb + 28)? as usize * 512,
            catalog: Vec::new(),
        };
        if volume.block_size == 0 || !volume.block_size.is_multiple_of(512) {
            return Err(corrupt(format!(
                "invalid allocation block size {}",
                volume.block_size
            )));
        }
        let catalog_fork = read_fork_record(image, mdb + 146, mdb + 150)?;
        volume.catalog = volume.read(&catalog_fork)?;
        Ok(volume)
    }

    /// All files in the catalog (from all folders).
    pub fn files(&self) -> Result<Vec<FileEntry>> {
        let cat = &self.catalog;
        let node_size = u16_at(cat, 14 + 18)? as usize;
        if node_size < 512 {
            return Err(corrupt("invalid catalog node size"));
        }
        let mut files = Vec::new();
        let mut node = u32_at(cat, 14 + 10)? as usize; // first leaf
        let mut visited = 0;
        while node != 0 {
            visited += 1;
            if visited > cat.len() / node_size {
                return Err(corrupt("loop in catalog leaf chain"));
            }
            let base = node * node_size;
            let records = u16_at(cat, base + 10)? as usize;
            for i in 0..records {
                let offset = u16_at(cat, base + node_size - 2 * (i + 1))? as usize;
                let record = base + offset;
                if let Some(file) = parse_file_record(cat, record)? {
                    files.push(file);
                }
            }
            node = u32_at(cat, base)? as usize; // forward link
        }
        Ok(files)
    }

    /// Find a file by name (ASCII case-insensitive, like HFS).
    pub fn file(&self, name: &str) -> Result<FileEntry> {
        self.files()?
            .into_iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
            .ok_or_else(|| Error::FileNotFound(name.to_string()))
    }

    pub fn data_fork(&self, file: &FileEntry) -> Result<Vec<u8>> {
        self.read(&file.data)
    }

    pub fn resource_fork(&self, file: &FileEntry) -> Result<Vec<u8>> {
        self.read(&file.resource)
    }

    fn read(&self, fork: &Fork) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(fork.len);
        for &(start, count) in &fork.extents {
            if out.len() >= fork.len {
                break;
            }
            let offset = self.first_block + start as usize * self.block_size;
            let len = count as usize * self.block_size;
            out.extend_from_slice(slice(self.image, offset, len)?);
        }
        if out.len() < fork.len {
            return Err(corrupt(
                "fork is fragmented beyond 3 extents (not supported)",
            ));
        }
        out.truncate(fork.len);
        Ok(out)
    }
}

/// Read a fork from a logical length field and a 3-extent record.
fn read_fork_record(data: &[u8], len_at: usize, extents_at: usize) -> Result<Fork> {
    let mut extents = [(0, 0); 3];
    for (i, extent) in extents.iter_mut().enumerate() {
        *extent = (
            u16_at(data, extents_at + i * 4)?,
            u16_at(data, extents_at + i * 4 + 2)?,
        );
    }
    Ok(Fork {
        len: u32_at(data, len_at)? as usize,
        extents,
    })
}

fn parse_file_record(cat: &[u8], record: usize) -> Result<Option<FileEntry>> {
    let key_len = u8_at(cat, record)? as usize;
    let name_len = u8_at(cat, record + 6)? as usize;
    let name = mac_roman(slice(cat, record + 7, name_len)?);
    // The record data starts after the key, aligned to an even offset.
    let mut data = record + key_len + 1;
    data += data % 2;
    if u8_at(cat, data)? != FILE_RECORD {
        return Ok(None);
    }
    let finfo = slice(cat, data + 4, 8)?;
    Ok(Some(FileEntry {
        name,
        file_type: finfo[0..4].try_into().unwrap(),
        creator: finfo[4..8].try_into().unwrap(),
        data: read_fork_record(cat, data + 26, data + 74)?,
        resource: read_fork_record(cat, data + 36, data + 86)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_hfs_images() {
        assert!(matches!(Volume::open(&[0; 2048]), Err(Error::NotHfs)));
        assert!(matches!(Volume::open(&[]), Err(Error::NotHfs)));
    }
}
