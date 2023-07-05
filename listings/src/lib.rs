#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Listing {
    pub flags: u8,
    pub city: u8,
    pub dye_id: u16,
    pub materia_ids: [u16; 5usize],
    pub amount: u16,
    pub price_per_unit: u32,
    pub retainer_name: [u8; 24usize],
}

pub const FLAGS_IS_HQ: u8 = 1 << 0;
pub const FLAGS_IS_CRAFTED: u8 = 1 << 1;
pub const FLAGS_IS_ON_MANNEQUIN: u8 = 1 << 2;

impl From<&universalis::Listing> for Listing {
    fn from(listing: &universalis::Listing) -> Self {
        Listing { 
            flags: 
                if listing.is_hq           { FLAGS_IS_HQ }           else { 0 } |
                if listing.is_crafted      { FLAGS_IS_CRAFTED }      else { 0 } |
                if listing.is_on_mannequin { FLAGS_IS_ON_MANNEQUIN } else { 0 },
            city: listing.city,
            dye_id: listing.dye_id,
            materia_ids: [
                if listing.materia.len() >= 1 { (listing.materia[0].materia_index as u16) << 8 | listing.materia[0].slot_index as u16 } else { 0 },
                if listing.materia.len() >= 2 { (listing.materia[1].materia_index as u16) << 8 | listing.materia[1].slot_index as u16 } else { 0 },
                if listing.materia.len() >= 3 { (listing.materia[2].materia_index as u16) << 8 | listing.materia[2].slot_index as u16 } else { 0 },
                if listing.materia.len() >= 4 { (listing.materia[3].materia_index as u16) << 8 | listing.materia[3].slot_index as u16 } else { 0 },
                if listing.materia.len() >= 5 { (listing.materia[4].materia_index as u16) << 8 | listing.materia[4].slot_index as u16 } else { 0 },
            ],
            amount: listing.amount,
            price_per_unit: listing.price_per_unit,
            retainer_name: {
                let mut array = [0; 24];
                array[..listing.retainer_name.len()].copy_from_slice(listing.retainer_name.as_bytes());
                array
            },
        }
    }
}

pub fn compress_listings(last_updated: i64, listings: &[Listing]) -> Vec<u8> {
    let mut input = Vec::<u8>::with_capacity(8 + listings.len() * std::mem::size_of::<Listing>());
    input.extend_from_slice(&last_updated.to_le_bytes());
    input.extend_from_slice(unsafe { std::slice::from_raw_parts(listings.as_ptr() as *const u8, listings.len() * std::mem::size_of::<Listing>()) });

    let output_max_size = zstd_safe::compress_bound(input.len());
    let mut output: Vec<u8> = Vec::with_capacity(output_max_size);

    let mut context = zstd_safe::CCtx::default();
    context.set_parameter(zstd_safe::CParameter::CompressionLevel(1)).unwrap();
    context.compress2(&mut output, &input).unwrap();

    output
}

pub fn decompress_listings(compressed: &[u8]) -> (i64, Vec<Listing>) {
    let mut output = Vec::with_capacity(8 + 100 * std::mem::size_of::<Listing>());
    zstd_safe::decompress(&mut output, compressed).unwrap();

    let last_updated = i64::from_le_bytes(output[0..8].try_into().unwrap());

    let mut listings = Vec::<Listing>::with_capacity(100);
    listings.extend_from_slice(unsafe { std::slice::from_raw_parts(output[8..].as_ptr() as *const Listing, (output.len() - 8) / std::mem::size_of::<Listing>()) });
    
    (last_updated, listings)
}