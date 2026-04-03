use crate::memory::types::{EngramError, EngramResult, MemoryId, MemoryNode};
use crc32fast::Hasher;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

/// Write-Ahead Log for crash-safe memory persistence.
///
/// Every mutation (store, update, delete) is first appended to the WAL
/// before being applied to in-memory structures. On recovery, the WAL
/// is replayed to reconstruct state.
///
/// Format per entry:
///   [4 bytes: payload length (u32 LE)]
///   [1 byte:  operation type]
///   [N bytes: bincode-encoded payload]
///   [4 bytes: CRC32 checksum of operation + payload]
pub struct WriteAheadLog {
    path: PathBuf,
    writer: BufWriter<File>,
    entry_count: u64,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WalOp {
    Store = 1,
    Update = 2,
    Delete = 3,
    Checkpoint = 4,
}

impl TryFrom<u8> for WalOp {
    type Error = EngramError;
    fn try_from(v: u8) -> EngramResult<Self> {
        match v {
            1 => Ok(Self::Store),
            2 => Ok(Self::Update),
            3 => Ok(Self::Delete),
            4 => Ok(Self::Checkpoint),
            _ => Err(EngramError::Wal(format!("Unknown WAL op: {}", v))),
        }
    }
}

#[derive(Debug)]
pub struct WalEntry {
    pub op: WalOp,
    pub payload: Vec<u8>,
}

impl WriteAheadLog {
    /// Open or create a WAL file
    pub fn open(path: impl AsRef<Path>) -> EngramResult<Self> {
        let path = path.as_ref().to_path_buf();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)?;

        let entry_count = Self::count_entries(&path)?;

        Ok(Self {
            path,
            writer: BufWriter::new(file),
            entry_count,
        })
    }

    /// Append a store operation
    pub fn log_store(&mut self, node: &MemoryNode) -> EngramResult<()> {
        let payload = bincode::serialize(node)
            .map_err(|e| EngramError::Serialization(e.to_string()))?;
        self.append(WalOp::Store, &payload)
    }

    /// Append an update operation
    pub fn log_update(&mut self, node: &MemoryNode) -> EngramResult<()> {
        let payload = bincode::serialize(node)
            .map_err(|e| EngramError::Serialization(e.to_string()))?;
        self.append(WalOp::Update, &payload)
    }

    /// Append a delete operation
    pub fn log_delete(&mut self, id: &MemoryId) -> EngramResult<()> {
        let payload = bincode::serialize(id)
            .map_err(|e| EngramError::Serialization(e.to_string()))?;
        self.append(WalOp::Delete, &payload)
    }

    /// Append a raw entry to the WAL
    fn append(&mut self, op: WalOp, payload: &[u8]) -> EngramResult<()> {
        let len = payload.len() as u32;

        // Compute CRC32 over op + payload
        let mut hasher = Hasher::new();
        hasher.update(&[op as u8]);
        hasher.update(payload);
        let checksum = hasher.finalize();

        // Write: length | op | payload | checksum
        self.writer.write_all(&len.to_le_bytes())?;
        self.writer.write_all(&[op as u8])?;
        self.writer.write_all(payload)?;
        self.writer.write_all(&checksum.to_le_bytes())?;
        self.writer.flush()?;

        self.entry_count += 1;
        Ok(())
    }

    /// Replay all entries from the WAL file
    pub fn replay(&self) -> EngramResult<Vec<WalEntry>> {
        Self::read_entries(&self.path)
    }

    /// Read all valid entries from a WAL file
    fn read_entries(path: &Path) -> EngramResult<Vec<WalEntry>> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut entries = Vec::new();

        loop {
            // Read length
            let mut len_buf = [0u8; 4];
            match reader.read_exact(&mut len_buf) {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }
            let len = u32::from_le_bytes(len_buf) as usize;

            // Read op
            let mut op_buf = [0u8; 1];
            reader.read_exact(&mut op_buf)?;
            let op = WalOp::try_from(op_buf[0])?;

            // Read payload
            let mut payload = vec![0u8; len];
            reader.read_exact(&mut payload)?;

            // Read checksum
            let mut crc_buf = [0u8; 4];
            reader.read_exact(&mut crc_buf)?;
            let stored_crc = u32::from_le_bytes(crc_buf);

            // Verify checksum
            let mut hasher = Hasher::new();
            hasher.update(&[op as u8]);
            hasher.update(&payload);
            let computed_crc = hasher.finalize();

            if stored_crc != computed_crc {
                tracing::warn!("WAL checksum mismatch, truncating at entry {}", entries.len());
                break;
            }

            entries.push(WalEntry { op, payload });
        }

        Ok(entries)
    }

    fn count_entries(path: &Path) -> EngramResult<u64> {
        Ok(Self::read_entries(path)?.len() as u64)
    }

    /// Write a checkpoint marker. On replay, entries before the last
    /// checkpoint can be skipped if indices were persisted at that point.
    pub fn log_checkpoint(&mut self) -> EngramResult<()> {
        let marker = b"checkpoint";
        self.append(WalOp::Checkpoint, marker)
    }

    /// Replay only entries after the last checkpoint marker.
    /// If no checkpoint exists, replays everything.
    pub fn replay_from_checkpoint(&self) -> EngramResult<Vec<WalEntry>> {
        let all = self.replay()?;
        // Find the last checkpoint position
        let last_cp = all.iter().rposition(|e| e.op == WalOp::Checkpoint);
        match last_cp {
            Some(pos) => Ok(all.into_iter().skip(pos + 1).collect()),
            None => Ok(all),
        }
    }

    /// Truncate the WAL (after snapshotting)
    pub fn truncate(&mut self) -> EngramResult<()> {
        drop(std::mem::replace(
            &mut self.writer,
            BufWriter::new(File::create(&self.path)?),
        ));
        self.entry_count = 0;
        Ok(())
    }

    pub fn entry_count(&self) -> u64 {
        self.entry_count
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::types::{MemoryNode, MemoryType};
    use tempfile::TempDir;
    use uuid::Uuid;

    #[test]
    fn test_wal_store_and_replay() {
        let dir = TempDir::new().unwrap();
        let wal_path = dir.path().join("test.wal");

        let agent = Uuid::now_v7();
        let node = MemoryNode::new(agent, MemoryType::Semantic, "test fact");

        // Write
        {
            let mut wal = WriteAheadLog::open(&wal_path).unwrap();
            wal.log_store(&node).unwrap();
            assert_eq!(wal.entry_count(), 1);
        }

        // Replay
        {
            let wal = WriteAheadLog::open(&wal_path).unwrap();
            let entries = wal.replay().unwrap();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].op, WalOp::Store);

            let recovered: MemoryNode = bincode::deserialize(&entries[0].payload).unwrap();
            assert_eq!(recovered.id, node.id);
            assert_eq!(recovered.content.as_str(), "test fact");
        }
    }

    #[test]
    fn test_wal_multiple_ops() {
        let dir = TempDir::new().unwrap();
        let wal_path = dir.path().join("test.wal");

        let agent = Uuid::now_v7();
        let node1 = MemoryNode::new(agent, MemoryType::Semantic, "fact one");
        let node2 = MemoryNode::new(agent, MemoryType::Semantic, "fact two");

        let mut wal = WriteAheadLog::open(&wal_path).unwrap();
        wal.log_store(&node1).unwrap();
        wal.log_store(&node2).unwrap();
        wal.log_delete(&node1.id).unwrap();

        let entries = wal.replay().unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].op, WalOp::Store);
        assert_eq!(entries[1].op, WalOp::Store);
        assert_eq!(entries[2].op, WalOp::Delete);
    }

    #[test]
    fn test_wal_truncate() {
        let dir = TempDir::new().unwrap();
        let wal_path = dir.path().join("test.wal");

        let agent = Uuid::now_v7();
        let node = MemoryNode::new(agent, MemoryType::Semantic, "fact");

        let mut wal = WriteAheadLog::open(&wal_path).unwrap();
        wal.log_store(&node).unwrap();
        wal.truncate().unwrap();
        assert_eq!(wal.entry_count(), 0);

        let entries = wal.replay().unwrap();
        assert_eq!(entries.len(), 0);
    }
}
