use std::{net::SocketAddr, time::Duration, thread::sleep};
use axum::{Router, routing::get, extract::Path, http::StatusCode, Json};
use time::format_description::well_known::Iso8601;
use tower_http::cors::{CorsLayer, Any};

fn get_time() -> String {
    time::OffsetDateTime::now_utc().format(&Iso8601::DEFAULT).unwrap()
}

const FLAGS_NUM_MATERIA_MASK: u8 = 0b00000111;
const FLAGS_IS_HQ: u8 = 0b00001000;
const FLAGS_IS_CRAFTED: u8 = 0b00010000;
const FLAGS_IS_DYED: u8 = 0b00100000;
const FLAGS_IS_ON_MANNEQUIN: u8 = 0b01000000;

fn convert_to_listingsdb_listing(listing: &universalis::Listing) -> listingsdb::Listing {
    let mut flags: u8 = listing.materia.len() as u8 & FLAGS_NUM_MATERIA_MASK;
    flags |= if listing.is_hq { FLAGS_IS_HQ } else { 0 };
    flags |= if listing.is_crafted { FLAGS_IS_CRAFTED } else { 0 };
    flags |= if listing.dye_id != 0 { FLAGS_IS_DYED } else { 0 };
    flags |= if listing.is_on_mannequin { FLAGS_IS_ON_MANNEQUIN } else { 0 };


    let retainer_name_length = std::cmp::min(listing.retainer_name.len(), listingsdb::LISTINGSDB_MAX_RETAINER_NAME_LENGTH as usize);
    let mut retainer_name = [0; listingsdb::LISTINGSDB_MAX_RETAINER_NAME_LENGTH as usize];
    retainer_name[..retainer_name_length].copy_from_slice(listing.retainer_name.as_bytes());

    let num_materia = std::cmp::min(listing.materia.len(), listingsdb::LISTINGSDB_MAX_NUM_MATERIA_PER_ITEM as usize);
    let mut materia_ids = [0; listingsdb::LISTINGSDB_MAX_NUM_MATERIA_PER_ITEM as usize];

    for (i, materia) in listing.materia[..num_materia].iter().enumerate() {
        materia_ids[i] = (materia.materia_index as u16) << 8 | (materia.slot_index as u16);
    }

    listingsdb::Listing {
        flags,
        city: listing.city,
        dye_id: listing.dye_id,
        materia_ids,
        amount: listing.amount,
        price_per_unit: listing.price_per_unit,
        retainer_name,
    }
}

#[derive(serde::Serialize)]
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
    retainer_name: [u8; listingsdb::LISTINGSDB_MAX_RETAINER_NAME_LENGTH as usize],
}

fn as_string<S>(retainer_name: &[u8; listingsdb::LISTINGSDB_MAX_RETAINER_NAME_LENGTH as usize], serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
    let str = match retainer_name.iter().any(|&x| x == 0) {
        false => unsafe { std::str::from_utf8_unchecked(retainer_name) },
        true => unsafe { std::ffi::CStr::from_ptr(retainer_name.as_ptr() as *const i8).to_str().unwrap() },
    };

    serializer.serialize_str(str)
}

fn convert_to_get_item_response_listing(listing: &listingsdb::Listing) -> GetItemResponseListing {
    GetItemResponseListing {
        is_hq: (listing.flags & FLAGS_IS_HQ) != 0,
        is_crafted: (listing.flags & FLAGS_IS_CRAFTED) != 0,
        is_on_mannequin: (listing.flags & FLAGS_IS_ON_MANNEQUIN) != 0,

        city: listing.city,
        dye_id: listing.dye_id,

        materia_ids: listing.materia_ids,

        amount: listing.amount,
        price_per_unit: listing.price_per_unit,

        retainer_name: listing.retainer_name,
    }
}

#[derive(serde::Serialize)]
struct GetItemResponse {
    last_updated: u64,
    listings: Vec<GetItemResponseListing>,
}

async fn get_item(Path((world, item)): Path<(u16, u16)>) -> Result<Json<GetItemResponse>, StatusCode> {
    println!("[{}] serving item: {:5} world: {:4}", get_time(), item, world);

    let mut listings: [listingsdb::Listing; 100] = unsafe { [std::mem::zeroed(); 100] };

    match listingsdb::get_listings(world, item, &mut listings) {
        Ok(result) => Ok(Json(GetItemResponse { 
            last_updated: result.0, 
            listings: listings[0..result.1 as usize]
                        .iter()
                        .map(convert_to_get_item_response_listing)
                        .collect::<Vec<GetItemResponseListing>>()
            })),
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

#[tokio::main]
async fn main() {
    let data_path = std::env::var("FFMARKETDB_DATA_PATH")
        .unwrap_or_else(|_| "data/listings".to_string());

    let bind_address = std::env::var("FFMARKETDB_BIND_ADDRESS")
        .unwrap_or_else(|_| "[::]:3000".to_string());
    
    listingsdb::init(&data_path).expect("Failed to init listingsdb.");
    let server_socket = bind_address.parse::<SocketAddr>().unwrap_or_else(|_| panic!("Failed to bind to FFMARKETDB_BIND_ADDRESS ({}).", bind_address));
    
    std::thread::spawn(|| {
        loop {
            let mut connection = match universalis::connect() {
                Ok(connection) => connection,
                Err(err) => { println!("[{}] Failed to connect to Universalis. Retrying. {:?}", get_time(), err); sleep(Duration::from_millis(5000)); continue },
            };
    
            connection.subscribe("listings/add");
    
            loop {
                let message = match connection.read_message() {
                    Ok(message) => message,
                    Err(_) => break,
                };
    
                match message {
                    universalis::Message::ListingsAdd { world, item, listings } => {
                        let mut db_listings: Vec<listingsdb::Listing> = Vec::with_capacity(listings.len());

                        for listing in listings {
                            db_listings.push(convert_to_listingsdb_listing(&listing));
                        }

                        listingsdb::update_listings(world, item, &db_listings);
                        println!("[{}] updated world: {:4} item: {:5}", get_time(), world, item);
                    },
                    _ => continue,
                };
            }
    
            println!("[{}] Websocket conection lost. Reconnecting.", get_time());
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
    });

    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any);

    let api_routes = Router::new()
        .route("/items/:world_id/:item_id", get(get_item))
        .layer(cors);

    axum::Server::bind(&server_socket)
        .serve(api_routes.into_make_service())
        .await
        .unwrap();
}
