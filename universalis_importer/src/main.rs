use tracing::{warn, error, info};
use redis::Commands;

fn main() {
    tracing_subscriber::fmt().init();

    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1".to_string());
    let redis_client = redis::Client::open(redis_url).expect("Invalid REDIS_URL. Exiting.");
    let redis_pool = r2d2::Pool::new(redis_client).expect("Failed to connect to redis db. Exiting.");

    loop {
        let mut connection = match universalis::Connection::connect() {
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
                universalis::Message::ListingsAdd { world, item, listings } => {
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
