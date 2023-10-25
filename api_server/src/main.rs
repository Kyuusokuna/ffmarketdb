use std::{env, net::SocketAddr};

use axum::{extract::{Path, State}, response::{IntoResponse, Response}};
use byteorder::{ReadBytesExt, LittleEndian};
use http::{Method, StatusCode};
use redis::AsyncCommands;
use serde_json::json;
use tower::ServiceBuilder;
use tower_http::{trace::TraceLayer, compression::CompressionLayer, cors::CorsLayer};

fn as_string(retainer_name: &[u8; 32usize]) -> &str {
    let str = match retainer_name.iter().any(|&x| x == 0) {
        false => unsafe { std::str::from_utf8_unchecked(retainer_name) },
        true => unsafe { std::ffi::CStr::from_ptr(retainer_name.as_ptr() as *const i8).to_str().unwrap() },
    };

    str
}

fn read_stored_data(compressed: &[u8]) -> Result<(i64, Vec<listings::Listing>), std::io::Error> {
    let mut uncompressed = Vec::with_capacity(/* num_listings */ 1 + listings::MAX_BYTES_PER_LISTING * listings::MAX_NUM_LISTINGS_PER_ITEM + /* timestamp */ 8);
    zstd_safe::decompress(&mut uncompressed, compressed).unwrap();

    let mut uncompressed = uncompressed.as_slice();
    let last_updated = uncompressed.read_i64::<LittleEndian>()?;
    let listings = listings::read_listings(uncompressed)?;

    Ok((last_updated, listings))
}

async fn get_item(Path((item, world)): Path<(u16, u16)>, State(mut redis_connection): State<redis::aio::ConnectionManager>) -> Response {//Result<Json<GetItemResponse>, StatusCode> {
    let compressed: Vec<u8> = match redis_connection.hget::<u16, u16, Option<Vec<u8>>>(item, world).await {
        Ok(Some(data)) => data,
        Ok(None) => match redis_connection.exists::<u16, bool>(item).await {
            Ok(true) => return (StatusCode::NOT_FOUND, "No data for this world.").into_response(),
            Ok(false) => return (StatusCode::NOT_FOUND, "No data for this item.").into_response(),
            Err(_) => return (StatusCode::NOT_FOUND, "No data for this world. But a database error occurred during existence check for item.").into_response(),
        },
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to get data from database.").into_response(),
    };

    let (last_updated, listings) = match read_stored_data(&compressed) {
        Ok(data) => data,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to decompress and parse stored listings.").into_response(),
    };
    
    let response = json!({
        "last_updated": last_updated,
        "listings": listings.iter().map(|listing| json!({
            "is_hq": listing.flags.contains(listings::ListingFlags::IS_HQ),
            "is_crafted": listing.flags.contains(listings::ListingFlags::IS_CRAFTED),
            "is_on_mannequin": listing.flags.contains(listings::ListingFlags::IS_ON_MANNEQUIN),
    
            "city": listing.city,
            "dye_id": listing.dye_id,
    
            "materia_ids": listing.materia_ids,
    
            "amount": listing.amount,
            "price_per_unit": listing.price_per_unit,
    
            "retainer_name": as_string(&listing.retainer_name),
        })).collect::<serde_json::Value>(),
    });

    (StatusCode::OK, response.to_string()).into_response()
}

#[tokio::main]
async fn main() {
    let bind_address = env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:3000".to_string());
    let redis_url: &str = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1".to_string()).leak();
    let redis_client = redis::Client::open(redis_url).unwrap_or_else(|_| panic!("Invalid REDIS_URL ({}). Exiting.", redis_url));
    let redis_connection = redis_client.get_tokio_connection_manager().await.unwrap_or_else(|_| panic!("Failed to connect to REDIS_URL({}). Exiting.", redis_url));

    let api_layers = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(CorsLayer::new().allow_methods(Method::GET).allow_origin(tower_http::cors::Any));

    let routes = axum::Router::new()
        .route("/items/:item_id/:world_id", axum::routing::get(get_item))
        .with_state(redis_connection)
        .layer(api_layers);

    let bind_address = bind_address.parse::<SocketAddr>().unwrap_or_else(|_| panic!("Failed to parse a valid address from BIND_ADDRESS ({bind_address}). Exiting."));
    axum::Server::bind(&bind_address)
        .serve(routes.into_make_service())
        .await
        .unwrap();
}
