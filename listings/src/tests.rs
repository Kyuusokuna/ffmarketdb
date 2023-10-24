#[cfg(test)]
use super::*;

#[test]
fn read_equals_written() {
    let mut buffer: Vec<u8> = vec![];
    let test_listings: Vec<Listing> = vec![Listing { 
        flags: ListingFlags::all(), 
        city: 1, 
        dye_id: 2, 
        materia_ids: [3, 4, 5, 6, 7], 
        amount: 8, 
        price_per_unit: 9, 
        retainer_name: [
            10, 11, 12, 13, 14, 15, 16, 17, 
            18, 19, 20, 21, 22, 23, 24, 25, 
            26, 27, 28, 29, 30, 31, 32, 33, 
            34, 35, 36, 37, 38, 39, 40, 41,
        ], 
    }];

    write_listings(&mut buffer, &test_listings).unwrap();
    let mut slice = buffer.as_slice();
    let read = read_listings(&mut slice).unwrap();
    assert_eq!(test_listings, read);
}