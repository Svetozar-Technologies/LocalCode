use crate::memory::types::EngramResult;
use memmap2::{Mmap, MmapMut, MmapOptions};
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};

/// Memory-mapped file region for zero-copy reads.
///
/// Provides safe wrappers around mmap for high-performance data access.
/// Used by the HNSW index and B-tree for sub-millisecond reads.
pub struct MappedFile {
    path: PathBuf,
    file: File,
    len: u64,
}

impl MappedFile {
    /// Open or create a memory-mapped file
    pub fn open(path: impl AsRef<Path>) -> EngramResult<Self> {
        let path = path.as_ref().to_path_buf();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)?;

        let len = file.metadata()?.len();

        Ok(Self { path, file, len })
    }

    /// Create a read-only memory map of the entire file
    pub fn map_read(&self) -> EngramResult<Option<Mmap>> {
        if self.len == 0 {
            return Ok(None);
        }
        let mmap = unsafe { MmapOptions::new().map(&self.file)? };
        Ok(Some(mmap))
    }

    /// Create a mutable memory map of the entire file
    pub fn map_mut(&self) -> EngramResult<Option<MmapMut>> {
        if self.len == 0 {
            return Ok(None);
        }
        let mmap = unsafe { MmapOptions::new().map_mut(&self.file)? };
        Ok(Some(mmap))
    }

    /// Extend the file and return a mutable map of the new region
    pub fn extend(&mut self, additional_bytes: u64) -> EngramResult<MmapMut> {
        let new_len = self.len + additional_bytes;
        self.file.set_len(new_len)?;

        let mmap = unsafe {
            MmapOptions::new()
                .offset(self.len)
                .len(additional_bytes as usize)
                .map_mut(&self.file)?
        };

        self.len = new_len;
        Ok(mmap)
    }

    /// Resize the file to exactly `new_len` bytes
    pub fn resize(&mut self, new_len: u64) -> EngramResult<()> {
        self.file.set_len(new_len)?;
        self.len = new_len;
        Ok(())
    }

    /// Current file size
    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Arena-style allocator over a memory-mapped file.
///
/// Provides bump allocation within a mapped region for zero-copy
/// storage of variable-length records.
pub struct MmapArena {
    file: MappedFile,
    write_offset: u64,
}

impl MmapArena {
    pub fn new(path: impl AsRef<Path>, initial_size: u64) -> EngramResult<Self> {
        let mut file = MappedFile::open(path)?;

        if file.is_empty() {
            file.resize(initial_size)?;
        }

        // Read write_offset from header (first 8 bytes)
        let write_offset = if file.len() >= 8 {
            if let Some(mmap) = file.map_read()? {
                u64::from_le_bytes(mmap[..8].try_into().unwrap_or([0; 8]))
            } else {
                8 // Start after header
            }
        } else {
            8
        };

        let write_offset = if write_offset < 8 { 8 } else { write_offset };

        Ok(Self { file, write_offset })
    }

    /// Allocate space and write data, returning the offset
    pub fn write(&mut self, data: &[u8]) -> EngramResult<u64> {
        let record_size = 4 + data.len() as u64; // 4 bytes for length prefix

        // Grow file if needed
        while self.write_offset + record_size > self.file.len() {
            let new_size = (self.file.len() * 2).max(self.file.len() + record_size + 4096);
            self.file.resize(new_size)?;
        }

        let offset = self.write_offset;

        // Write length + data using mmap
        if let Some(mut mmap) = self.file.map_mut()? {
            let off = offset as usize;
            let len = data.len() as u32;
            mmap[off..off + 4].copy_from_slice(&len.to_le_bytes());
            mmap[off + 4..off + 4 + data.len()].copy_from_slice(data);

            // Update write offset in header
            self.write_offset = offset + record_size;
            mmap[..8].copy_from_slice(&self.write_offset.to_le_bytes());

            mmap.flush()?;
        }

        Ok(offset)
    }

    /// Read data at a given offset
    pub fn read(&self, offset: u64) -> EngramResult<Option<Vec<u8>>> {
        let mmap = match self.file.map_read()? {
            Some(m) => m,
            None => return Ok(None),
        };

        let off = offset as usize;
        if off + 4 > mmap.len() {
            return Ok(None);
        }

        let len = u32::from_le_bytes(mmap[off..off + 4].try_into().unwrap()) as usize;
        if off + 4 + len > mmap.len() {
            return Ok(None);
        }

        Ok(Some(mmap[off + 4..off + 4 + len].to_vec()))
    }

    pub fn used_bytes(&self) -> u64 {
        self.write_offset
    }

    pub fn total_bytes(&self) -> u64 {
        self.file.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_mapped_file_basic() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.dat");

        let mut mf = MappedFile::open(&path).unwrap();
        assert!(mf.is_empty());

        mf.resize(1024).unwrap();
        assert_eq!(mf.len(), 1024);

        // Write via mmap
        if let Some(mut mmap) = mf.map_mut().unwrap() {
            mmap[0..5].copy_from_slice(b"hello");
            mmap.flush().unwrap();
        }

        // Read via mmap
        if let Some(mmap) = mf.map_read().unwrap() {
            assert_eq!(&mmap[0..5], b"hello");
        }
    }

    #[test]
    fn test_mmap_arena() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("arena.dat");

        let mut arena = MmapArena::new(&path, 4096).unwrap();

        let offset1 = arena.write(b"first record").unwrap();
        let offset2 = arena.write(b"second record").unwrap();

        assert_ne!(offset1, offset2);

        let data1 = arena.read(offset1).unwrap().unwrap();
        assert_eq!(data1, b"first record");

        let data2 = arena.read(offset2).unwrap().unwrap();
        assert_eq!(data2, b"second record");
    }

    #[test]
    fn test_arena_persistence() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("persist.dat");

        let offset;
        {
            let mut arena = MmapArena::new(&path, 4096).unwrap();
            offset = arena.write(b"persistent data").unwrap();
        }

        // Reopen
        let arena = MmapArena::new(&path, 4096).unwrap();
        let data = arena.read(offset).unwrap().unwrap();
        assert_eq!(data, b"persistent data");
    }
}
