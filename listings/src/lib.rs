use bitflags::bitflags;

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
    pub struct ListingFlags : u8 {
        const IS_HQ           = 0b0000_0001;
        const IS_CRAFTED      = 0b0000_0010;
        const IS_ON_MANNEQUIN = 0b0000_0100;
    }
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Listing {
    pub flags: ListingFlags,
    pub city: u8,
    pub dye_id: u16,
    pub materia_ids: [u16; 5usize],
    pub amount: u16,
    pub price_per_unit: u32,
    pub retainer_name: [u8; 24usize],
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