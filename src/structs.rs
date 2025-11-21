use crate::prelude::*;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct MapHeader {
    pub magic: u32, // 0xDEADBEEF
    pub version: u32,
    pub entry_count: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BlockHeader {
    pub compression_type: u8,
    pub compressed_size: u32,
    pub decompressed_size: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TableHeader {
    pub entry_count: u64,
    pub value_count: u64,
    pub timestamp: u64,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct TableEntry {
    pub id: u64,
    pub offset: u32,
    pub length: u32,
}
