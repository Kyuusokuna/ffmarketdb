use byteorder::{WriteBytesExt, LittleEndian};
use redis::{Commands, ConnectionLike};
use tracing::{warn, error, info};

mod websocket;

impl From<&websocket::Listing> for listings::Listing {
    fn from(listing: &websocket::Listing) -> Self {
        listings::Listing { 
            flags: 
                if listing.is_hq           { listings::ListingFlags::IS_HQ }           else { listings::ListingFlags::empty() } |
                if listing.is_crafted      { listings::ListingFlags::IS_CRAFTED }      else { listings::ListingFlags::empty() } |
                if listing.is_on_mannequin { listings::ListingFlags::IS_ON_MANNEQUIN } else { listings::ListingFlags::empty() },
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
                let mut array = [0; 32];
                array[..listing.retainer_name.len()].copy_from_slice(listing.retainer_name.as_bytes());
                array
            },
        }
    }
}

fn make_stored_data(last_updated: i64, listings: &[websocket::Listing]) -> Result<Vec<u8>, std::io::Error>{
    let mut uncompressed = Vec::<u8>::with_capacity(/* num_listings */ 1 + listings::MAX_BYTES_PER_LISTING * listings.len() + /* timestamp */ 8);
    uncompressed.write_i64::<LittleEndian>(last_updated)?;
    listings::write_listings(&mut uncompressed, listings)?;

    let mut context = zstd_safe::CCtx::default();
    context.set_parameter(zstd_safe::CParameter::CompressionLevel(1)).unwrap();

    let mut compressed = Vec::<u8>::with_capacity(zstd_safe::compress_bound(uncompressed.len()));
    context.compress2(&mut compressed, &uncompressed).unwrap();

    Ok(compressed)
}

fn main() {
    tracing_subscriber::fmt().init();

    let redis_url: &str = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1".to_string()).leak();
    let redis_client = redis::Client::open(redis_url).unwrap_or_else(|_| panic!("Invalid REDIS_URL ({}). Exiting.", redis_url));
    let mut redis_connection = redis_client.get_connection().unwrap_or_else(|_| panic!("Failed to connect to REDIS_URL({}). Exiting.", redis_url));

    loop {
        let mut connection = match websocket::Connection::connect() {
            Ok(connection) => connection,
            Err(_) => {
                error!("Failed to connect to universalis. Retrying.");
                continue;
            },
        };

        info!("Connected to universalis");
        connection.subscribe("listings/add");

        loop {
            let message = match connection.read_message() {
                Ok(message) => message,
                Err(_) => break,
            };

            match message {
                websocket::Message::ListingsAdd { world, item, listings } => {
                    let data = match make_stored_data(time::OffsetDateTime::now_utc().unix_timestamp(), &listings) {
                        Ok(data) => data,
                        Err(_) => {
                            error!("Failed to convert data received from universalis to the storage format. Dropping data.");
                            continue;
                        }
                    };

                    let key = format!("{item}");
                    match redis_connection.hset(key, world, data) {
                        Ok(()) => (),
                        Err(_) => {
                            error!("Failed to set redis key. Dropping data.");

                            if !redis_connection.check_connection() {
                                redis_connection = match redis_client.get_connection() {
                                    Ok(connection) => connection,
                                    Err(_) => {
                                        error!("Failed to connect to REDIS_URL({}). Retrying.", redis_url);
                                        continue;
                                    },
                                };
                            }

                            continue;
                        },
                    }

                    info!("updated {:3} listings for world: {world:4} item: {item:5}", listings.len());
                },
                _ => continue,
            }
        }

        warn!("Connection with universalis lost. Reconnecting.");
    }
}
