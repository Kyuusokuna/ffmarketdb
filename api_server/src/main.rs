use std::{env, net::SocketAddr};

use axum::{extract::Path, Json};
use byteorder::{ReadBytesExt, LittleEndian};
use http::{Method, StatusCode};
use redis::Commands;
use serde::{Serialize, Serializer};
use tower::ServiceBuilder;
use tower_http::{trace::TraceLayer, compression::CompressionLayer, cors::CorsLayer};


#[derive(Serialize)]
struct GetItemResponseListing {
    is_hq: bool,
    is_crafted: bool,
    is_on_mannequin: bool,

    city: u8,
    dye_id: u16,

    materia_ids: [u16; 5usize],

    amount: u16,
    price_per_unit: u32,

    #[serde(serialize_with = "as_string")]
    retainer_name: [u8; 32usize],
}

#[derive(Serialize)]
struct GetItemResponse {
    last_updated: i64,
    listings: Vec<GetItemResponseListing>,
}

fn as_string<S>(retainer_name: &[u8; 32usize], serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    let str = match retainer_name.iter().any(|&x| x == 0) {
        false => unsafe { std::str::from_utf8_unchecked(retainer_name) },
        true => unsafe { std::ffi::CStr::from_ptr(retainer_name.as_ptr() as *const i8).to_str().unwrap() },
    };

    serializer.serialize_str(str)
}

fn convert_to_get_item_response_listing(listing: &listings::Listing) -> GetItemResponseListing {
    GetItemResponseListing {
        is_hq: listing.flags.contains(listings::ListingFlags::IS_HQ),
        is_crafted: listing.flags.contains(listings::ListingFlags::IS_CRAFTED),
        is_on_mannequin: listing.flags.contains(listings::ListingFlags::IS_ON_MANNEQUIN),

        city: listing.city,
        dye_id: listing.dye_id,

        materia_ids: listing.materia_ids,

        amount: listing.amount,
        price_per_unit: listing.price_per_unit,

        retainer_name: listing.retainer_name,
    }
}

fn read_stored_data(compressed: &[u8]) -> Result<(i64, Vec<listings::Listing>), std::io::Error> {
    let mut uncompressed = Vec::with_capacity(/* num_listings */ 1 + listings::MAX_BYTES_PER_LISTING * listings::MAX_NUM_LISTINGS_PER_ITEM + /* timestamp */ 8);
    zstd_safe::decompress(&mut uncompressed, compressed).unwrap();

    let mut uncompressed = uncompressed.as_slice();
    let last_updated = uncompressed.read_i64::<LittleEndian>()?;
    let listings = listings::read_listings(uncompressed)?;

    Ok((last_updated, listings))
}

async fn get_item(Path((item, world)): Path<(u16, u16)>) -> Result<Json<GetItemResponse>, StatusCode> {
    let client = redis::Client::open(env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1".to_string())).expect("Failed to parse REDIS_URL.");
    let mut connection = client.get_connection().expect("Failed to get connetion to redis.");

    let compressed :Vec<u8> = connection.hget(item, world).expect("Failed to get listings");
    let (last_updated, listings) = read_stored_data(&compressed).expect("Failed to parse db entry.");

    Ok(Json(GetItemResponse{
        last_updated,
        listings: listings.iter().map(convert_to_get_item_response_listing).collect()
    }))
}

#[tokio::main]
async fn main() {
    let bind_address = env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1".to_string());

    let api_layers = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::new().allow_methods(Method::GET).allow_origin(tower_http::cors::Any));

    let routes = axum::Router::new()
        .route("/items/:item_id/:world_id", axum::routing::get(get_item))
        .layer(api_layers);

    let bind_address = bind_address.parse::<SocketAddr>().unwrap_or_else(|_| panic!("Failed to parse a valid address from BIND_ADDRESS ({bind_address})."));
    axum::Server::bind(&bind_address)
        .serve(routes.into_make_service())
        .await
        .unwrap();
}
