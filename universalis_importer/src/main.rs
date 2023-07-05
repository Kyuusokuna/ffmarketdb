use tracing::{warn, error, info};
use redis::Commands;

mod websocket;

impl From<&websocket::Listing> for listings::Listing {
    fn from(listing: &websocket::Listing) -> Self {
        listings::Listing { 
            flags: 
                if listing.is_hq           { listings::FLAGS_IS_HQ }           else { 0 } |
                if listing.is_crafted      { listings::FLAGS_IS_CRAFTED }      else { 0 } |
                if listing.is_on_mannequin { listings::FLAGS_IS_ON_MANNEQUIN } else { 0 },
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

fn main() {
    tracing_subscriber::fmt().init();

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1".to_string());
    let redis_client = redis::Client::open(redis_url).expect("Invalid REDIS_URL. Exiting.");
    let redis_pool = r2d2::Pool::new(redis_client).expect("Failed to connect to redis db. Exiting.");

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
                    info!("updating {:3} listings for world: {world:4} item: {item:5}", listings.len());

                    let mut redis = match redis_pool.get() {
                        Ok(redis) => redis,
                        Err(_) => {
                            error!("Failed to get a redis connection. Dropping data.");
                            continue;
                        },
                    };

                    let key = format!("{item}");
                    match redis.hset(key, world, listings::compress_listings(time::OffsetDateTime::now_utc().unix_timestamp(), &listings.iter().map(|x| x.into()).collect::<Vec<listings::Listing>>())) {
                        Ok(()) => (),
                        Err(_) => {
                            error!("Failed to set redis key. Dropping data.");
                            continue;
                        },
                    }
                },
                _ => continue,
            }
        }

        warn!("Connection with universalis lost. Reconnecting.");
    }
}
