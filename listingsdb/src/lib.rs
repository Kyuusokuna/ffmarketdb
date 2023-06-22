pub use listingsdb_sys::ListingsDB_Listing as Listing;
pub use listingsdb_sys::LISTINGSDB_MAX_NUM_LISTINGS_PER_ITEM;
pub use listingsdb_sys::LISTINGSDB_MAX_NUM_MATERIA_PER_ITEM;
pub use listingsdb_sys::LISTINGSDB_MAX_RETAINER_NAME_LENGTH;
use std::{ffi::CString, fs::create_dir_all};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to initialize listingsdb.")]
    InitError,
    #[error("Item not found.")]
    ItemNotFoundError,
}

pub fn init(base_path: &str) -> Result<(), Error> {
    _ = create_dir_all(base_path);
    let path = CString::new(base_path).unwrap();
    
    unsafe {
        if !listingsdb_sys::ListingsDB_init(path.as_ptr()) {
            return Err(Error::InitError);
        } else {
            return Ok(());
        }
    }
}

pub fn shutdown() {
    unsafe {
        listingsdb_sys::ListingsDB_shutdown();
    }
}

pub fn update_listings(world_id: u16, item_id: u16, new_listings: &[Listing]) {
    unsafe {
        listingsdb_sys::ListingsDB_update_listings(world_id, item_id, new_listings.len() as u8, new_listings.as_ptr());
    }
}

pub fn get_listings(world_id: u16, item_id: u16, listings: &mut [Listing; 100]) -> Result<(u64, u8), Error> {
    let mut last_update_time: u64 = 0;
    let mut num_listings: u8 = 0;

    unsafe {
        if !listingsdb_sys::ListingsDB_get_listings(world_id, item_id, &mut num_listings, listings.as_mut_ptr(), &mut last_update_time) {
            return Err(Error::ItemNotFoundError);
        } else {
            return Ok((last_update_time, num_listings));
        }
    }
}