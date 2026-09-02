use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedRegion {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl SelectedRegion {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
}

pub static ACTIVE_REGION: Mutex<Option<SelectedRegion>> = Mutex::new(None);

pub fn set_active_region(region: Option<SelectedRegion>) {
    if let Ok(mut lock) = ACTIVE_REGION.lock() {
        *lock = region;
        if let Some(r) = region {
            println!("[RegionEngine] Active capture region set to: {}x{} at ({}, {})", r.width, r.height, r.x, r.y);
        } else {
            println!("[RegionEngine] Active capture region cleared (Full Screen)");
        }
    }
}

pub fn get_active_region() -> Option<SelectedRegion> {
    if let Ok(lock) = ACTIVE_REGION.lock() {
        *lock
    } else {
        None
    }
}
