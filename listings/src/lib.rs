use std::io::{Result, Write, Read};
use bitflags::bitflags;
use byteorder::{WriteBytesExt, LittleEndian, ReadBytesExt};

#[cfg(test)]
mod tests;

pub const MAX_NUM_LISTINGS_PER_ITEM: usize = 100;
pub const MAX_BYTES_PER_LISTING: usize = 52;
pub const MAX_BYTES_PER_LISTINGS: usize = MAX_BYTES_PER_LISTING * MAX_NUM_LISTINGS_PER_ITEM + 1;

bitflags! {
    #[repr(transparent)]
    #[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
    pub struct ListingFlags : u8 {
        const IS_HQ           = 0b0000_0001;
        const IS_CRAFTED      = 0b0000_0010;
        const IS_ON_MANNEQUIN = 0b0000_0100;
    }
}

/// A single listing for an item
#[repr(C)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct Listing {
    pub flags: ListingFlags,
    pub city: u8,

    pub dye_id: u16,
    pub materia_ids: [u16; 5usize],

    pub amount: u16,
    pub price_per_unit: u32,

    pub retainer_name: [u8; 32usize],
}

impl From<&Listing> for Listing {
    fn from(val: &Listing) -> Self {
        *val
    }
}

pub fn write_listings<B: Write, T>(buffer: &mut B, listings: &[T]) -> Result<()> 
where for<'a> &'a T: Into<Listing> {
    buffer.write_u8(listings.len() as u8)?;

    for listing in listings {
        let listing = Into::<Listing>::into(listing);

        buffer.write_u8(listing.flags.bits())?;
        buffer.write_u8(listing.city)?;

        buffer.write_u16::<LittleEndian>(listing.amount)?;
        buffer.write_u32::<LittleEndian>(listing.price_per_unit)?;

        buffer.write_u16::<LittleEndian>(listing.dye_id)?;

        for materia in listing.materia_ids {
            buffer.write_u16::<LittleEndian>(materia)?;
        }

        for char in listing.retainer_name {
            buffer.write_u8(char)?;
        }
    }

    Ok(())
}

pub fn read_listings<B: Read>(mut buffer: B) -> Result<Vec<Listing>> {
    let num = buffer.read_u8()?;

    let mut result = Vec::<Listing>::with_capacity(num.into());
    for _ in 0..num {
        let listing = Listing { 
            flags:          ListingFlags::from_bits_retain(buffer.read_u8()?), 
            city:           buffer.read_u8()?, 

            amount:         buffer.read_u16::<LittleEndian>()?, 
            price_per_unit: buffer.read_u32::<LittleEndian>()?, 

            dye_id:         buffer.read_u16::<LittleEndian>()?, 

            materia_ids: [
                buffer.read_u16::<LittleEndian>()?,
                buffer.read_u16::<LittleEndian>()?,
                buffer.read_u16::<LittleEndian>()?,
                buffer.read_u16::<LittleEndian>()?,
                buffer.read_u16::<LittleEndian>()?
            ], 

            retainer_name: [
                buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, 
                buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, 
                buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, 
                buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, 
                buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, 
                buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, 
                buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, 
                buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, buffer.read_u8()?, 
            ], 
        };

        result.push(listing);
    }

    Ok(result)
}
