// src/wallet/cache.rs
//
// A BlockCache implementation for ZecLedger, wrapping FsBlockDb in a Mutex
// (FsBlockDb holds a RefCell SQLite connection and is not Sync on its own).

use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use prost::Message;

use zcash_client_backend::data_api::chain::{error, BlockCache, BlockSource};
use zcash_client_backend::data_api::scanning::ScanRange;
use zcash_client_backend::proto::compact_formats::CompactBlock;
use zcash_client_sqlite::chain::BlockMeta;
use zcash_client_sqlite::{FsBlockDb, FsBlockDbError};
use zcash_primitives::block::BlockHash;
use zcash_protocol::consensus::BlockHeight;

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("block cache error: {0}")]
    Fs(FsBlockDbError),
    #[error("io error writing block file: {0}")]
    Io(std::io::Error),
    #[error("cache lock poisoned")]
    Lock,
}

impl From<FsBlockDbError> for CacheError {
    fn from(e: FsBlockDbError) -> Self {
        CacheError::Fs(e)
    }
}

pub struct ZecLedgerCache {
    inner: Mutex<FsBlockDb>,
    blocks_dir: PathBuf,
}

impl ZecLedgerCache {
    pub fn new(inner: FsBlockDb, blocks_dir: PathBuf) -> Self {
        Self {
            inner: Mutex::new(inner),
            blocks_dir,
        }
    }
}

impl BlockSource for ZecLedgerCache {
    type Error = CacheError;

    fn with_blocks<F, WalletErrT>(
        &self,
        from_height: Option<BlockHeight>,
        limit: Option<usize>,
        mut with_block: F,
    ) -> Result<(), error::Error<WalletErrT, Self::Error>>
    where
        F: FnMut(CompactBlock) -> Result<(), error::Error<WalletErrT, Self::Error>>,
    {
        let guard = self
            .inner
            .lock()
            .map_err(|_| error::Error::BlockSource(CacheError::Lock))?;
        let mut callback_err: Option<error::Error<WalletErrT, Self::Error>> = None;
        let result =
            guard.with_blocks::<_, WalletErrT>(from_height, limit, |cb| match with_block(cb) {
                Ok(()) => Ok(()),
                Err(e) => {
                    callback_err = Some(e);
                    Err(error::Error::BlockSource(FsBlockDbError::CorruptedData(
                        "callback halted".to_string(),
                    )))
                }
            });
        if let Some(e) = callback_err {
            return Err(e);
        }
        result.map_err(|e| match e {
            error::Error::BlockSource(s) => error::Error::BlockSource(CacheError::Fs(s)),
            _ => error::Error::BlockSource(CacheError::Fs(FsBlockDbError::CorruptedData(
                "unexpected".to_string(),
            ))),
        })
    }
}

#[async_trait]
impl BlockCache for ZecLedgerCache {
    fn get_tip_height(
        &self,
        _range: Option<&ScanRange>,
    ) -> Result<Option<BlockHeight>, Self::Error> {
        let guard = self.inner.lock().map_err(|_| CacheError::Lock)?;
        Ok(guard.get_max_cached_height()?)
    }

    async fn read(&self, range: &ScanRange) -> Result<Vec<CompactBlock>, Self::Error> {
        let start = range.block_range().start;
        let limit = (range.block_range().end - start) as usize;
        let mut blocks = Vec::new();
        {
            let guard = self.inner.lock().map_err(|_| CacheError::Lock)?;
            guard
                .with_blocks::<_, ()>(Some(start), Some(limit), |cb| {
                    blocks.push(cb);
                    Ok(())
                })
                .map_err(|_| CacheError::Fs(FsBlockDbError::CacheMiss(start)))?;
        }
        Ok(blocks)
    }

    async fn insert(&self, compact_blocks: Vec<CompactBlock>) -> Result<(), Self::Error> {
        let mut metas = Vec::with_capacity(compact_blocks.len());
        for cb in &compact_blocks {
            let meta = BlockMeta {
                height: BlockHeight::from_u32(cb.height as u32),
                block_hash: BlockHash::from_slice(&cb.hash),
                block_time: cb.time,
                sapling_outputs_count: cb.vtx.iter().map(|tx| tx.outputs.len() as u32).sum(),
                orchard_actions_count: cb.vtx.iter().map(|tx| tx.actions.len() as u32).sum(),
            };
            let path = meta.block_file_path(&self.blocks_dir);
            let bytes = cb.encode_to_vec();
            let mut f = std::fs::File::create(&path).map_err(CacheError::Io)?;
            f.write_all(&bytes).map_err(CacheError::Io)?;
            metas.push(meta);
        }
        let guard = self.inner.lock().map_err(|_| CacheError::Lock)?;
        guard.write_block_metadata(&metas)?;
        Ok(())
    }

    async fn delete(&self, range: ScanRange) -> Result<(), Self::Error> {
        let start = range.block_range().start;
        let target = if u32::from(start) == 0 {
            start
        } else {
            start - 1
        };
        let guard = self.inner.lock().map_err(|_| CacheError::Lock)?;
        guard.truncate_to_height(target)?;
        Ok(())
    }
}
