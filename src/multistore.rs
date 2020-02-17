// mostly a copy of https://github.com/filecoin-project/merkle_light/blob/master/src/store/vec.rs

use std::ops::Range;

use anyhow::Result;

use merkletree::merkle::Element;
use merkletree::store::{DiskStore, Store, StoreConfig, VecStore};

#[derive(Debug)]
pub struct MultiStore<E: Element> {
    disk: DiskStore<E>,
    mem: VecStore<E>,
}

const DISK_MAX: usize = 262144 * 48; // ~375Mb

impl<E: Element> Store<E> for MultiStore<E> {
    fn new_with_config(size: usize, _config: StoreConfig) -> Result<Self> {
        Self::new(size)
    }

    fn new(size: usize) -> Result<Self> {
        Ok(MultiStore {
            disk: DiskStore::new(DISK_MAX).unwrap(),
            mem: VecStore::new(size - DISK_MAX).unwrap(),
        })
    }

    fn write_at(&mut self, el: E, index: usize) -> Result<()> {
        if index > DISK_MAX {
            self.mem.write_at(el, index - DISK_MAX)
        } else {
            self.disk.write_at(el, index)
        }
    }

    fn copy_from_slice(&mut self, buf: &[u8], start: usize) -> Result<()> {
        if start + (buf.len() / E::byte_len()) > DISK_MAX {
            self.mem.copy_from_slice(buf, start - DISK_MAX)
        } else {
            self.disk.copy_from_slice(buf, start)
        }
    }

    fn new_from_slice_with_config(size: usize, data: &[u8], _config: StoreConfig) -> Result<Self> {
        Self::new_from_slice(size, &data)
    }

    fn new_from_slice(_size: usize, _data: &[u8]) -> Result<Self> {
        unimplemented!("nope, too hard");
    }

    fn new_from_disk(_size: usize, _config: &StoreConfig) -> Result<Self> {
        unimplemented!("Cannot load a MultiStore from disk");
    }

    fn read_at(&self, index: usize) -> Result<E> {
        if index > DISK_MAX {
            self.mem.read_at(index - DISK_MAX)
        } else {
            self.disk.read_at(index)
        }
    }

    fn read_into(&self, index: usize, buf: &mut [u8]) -> Result<()> {
        if index > DISK_MAX {
            self.mem.read_into(index - DISK_MAX, buf)
        } else {
            self.disk.read_into(index, buf)
        }
    }

    fn read_range_into(&self, _start: usize, _end: usize, _buf: &mut [u8]) -> Result<()> {
        unimplemented!("Not required here");
    }

    fn read_range(&self, r: Range<usize>) -> Result<Vec<E>> {
        if r.start > DISK_MAX {
            // entire range is in mem
            let nr = Range {
                start: r.start - DISK_MAX,
                end: r.end - DISK_MAX,
            };
            self.mem.read_range(nr)
        } else if r.end > DISK_MAX {
            // split across disk and mem
            let nrdisk = Range {
                start: r.start,
                end: DISK_MAX,
            };
            let nrmem = Range {
                start: 0,
                end: r.end - DISK_MAX,
            };
            let rdisk = self.mem.read_range(nrdisk).unwrap();
            let rmem = self.mem.read_range(nrmem).unwrap();
            let mut rv = Vec::with_capacity(r.end - r.start);
            rv.extend(rdisk);
            rv.extend(rmem);
            Ok(rv)
        } else {
            // entire range is in disk
            self.disk.read_range(r)
        }
    }

    fn len(&self) -> usize {
        self.disk.len() + self.mem.len()
    }

    fn loaded_from_disk(&self) -> bool {
        false
    }

    fn compact(&mut self, _config: StoreConfig, _store_version: u32) -> Result<bool> {
        Ok(true)
    }

    fn delete(_config: StoreConfig) -> Result<()> {
        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.disk.is_empty() && self.mem.is_empty()
    }

    fn push(&mut self, el: E) -> Result<()> {
        if self.disk.len() > DISK_MAX {
            self.mem.push(el)
        } else {
            self.disk.push(el)
        }
    }
}
