//! The allocator of thread IDs.

use std::sync::Mutex;
use structures::{
    error::LxError,
    thread::{TID_MAX, TID_MIN},
};

static TID_ALLOC: Mutex<TidAlloc> = Mutex::new(TidAlloc::new());

#[derive(Debug)]
pub struct TidAlloc {
    maps: Vec<TidMap>,
    last_alloc: i32,
    tid_max: i32,
}
impl TidAlloc {
    pub const fn new() -> Self {
        Self {
            maps: Vec::new(),
            last_alloc: TID_MIN - 1,
            tid_max: TID_MAX,
        }
    }

    pub fn alloc(&mut self) -> Result<i32, LxError> {
        let mut search_scratch = false;
        let allocated = 'outer: loop {
            let search_from = match search_scratch {
                true => TID_MIN,
                false => self.last_alloc + 1,
            };
            for i in search_from..=self.tid_max {
                if !self.get(i) {
                    self.set(i);
                    break 'outer i;
                }
            }
            if search_scratch {
                return Err(LxError::EAGAIN);
            }
            search_scratch = true;
        };
        self.last_alloc = allocated;
        Ok(allocated)
    }

    pub fn dealloc(&mut self, value: i32) {
        self.unset(value);
    }

    pub fn get(&self, value: i32) -> bool {
        let (nmap, byte, shift) = Self::position(value);
        let Some(map) = self.maps.get(nmap) else {
            return false;
        };
        (map.bitmap[byte] & (1 << shift)) != 0
    }

    pub fn set(&mut self, value: i32) {
        let (nmap, byte, shift) = Self::position(value);
        let Some(map) = self.maps.get_mut(nmap) else {
            self.maps.resize_with(nmap + 1, TidMap::new);
            self.set(value);
            return;
        };
        if (map.bitmap[byte] & (1 << shift)) == 0 {
            map.free_count -= 1;
        }
        map.bitmap[byte] |= 1 << shift;
    }

    pub fn unset(&mut self, value: i32) {
        let (nmap, byte, shift) = Self::position(value);
        let Some(map) = self.maps.get_mut(nmap) else {
            return;
        };
        if (map.bitmap[byte] & (1 << shift)) != 0 {
            map.free_count += 1;
        }
        map.bitmap[byte] &= (1 << shift) ^ 0xff;
    }

    const fn position(value: i32) -> (usize, usize, u32) {
        debug_assert!(value >= TID_MIN);
        let pure_value = value - TID_MIN;
        let nmap = pure_value / TidMap::CAPACITY as i32;
        let map_offset = pure_value % TidMap::CAPACITY as i32;
        let byte = map_offset / 4096;
        let shift = map_offset % 4096;
        (nmap as _, byte as _, shift as _)
    }
}

#[derive(Debug)]
struct TidMap {
    bitmap: Box<[u8; 4096]>,
    free_count: usize,
}
impl TidMap {
    const CAPACITY: usize = 4096 * 8;

    pub fn new() -> Self {
        Self {
            bitmap: Box::new([0; _]),
            free_count: Self::CAPACITY,
        }
    }
}
impl Default for TidMap {
    fn default() -> Self {
        Self::new()
    }
}

pub fn alloc() -> Result<i32, LxError> {
    TID_ALLOC.lock().unwrap().alloc()
}

pub fn dealloc(value: i32) {
    TID_ALLOC.lock().unwrap().dealloc(value);
}
