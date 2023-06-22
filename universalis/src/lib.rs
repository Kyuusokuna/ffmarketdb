use std::{ net::{ TcpStream, ToSocketAddrs}, time::Duration };
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Deserialize)]
pub struct MateriaInfo {
    #[serde(rename = "slotID")]
    pub slot_index: u8,
    #[serde(rename = "materiaID")]
    pub materia_index: u8,
}

#[derive(Debug, Deserialize)]
pub struct Listing {
    #[serde(rename = "hq")]
    pub is_hq: bool,
    #[serde(rename = "isCrafted")]
    pub is_crafted: bool,
    #[serde(rename = "onMannequin")]
    pub is_on_mannequin: bool,

    #[serde(rename = "retainerCity")]
    pub city: u8,
    #[serde(rename = "retainerName")]
    pub retainer_name: String,

    #[serde(rename = "stainID")]
    pub dye_id: u16,
    #[serde(rename = "materia")]
    pub materia: Vec<MateriaInfo>,

    #[serde(rename = "quantity")]
    pub amount: u16,
    #[serde(rename = "pricePerUnit")]
    pub price_per_unit: u32,
    #[serde(rename = "total")]
    pub total_price: u32,

    #[serde(rename = "lastReviewTime")]
    pub creation_time: u64,    

    #[serde(rename = "sellerID")]
    pub seller_id: String,
}

#[derive(Debug, Deserialize)]
pub struct Sale {
    #[serde(rename = "hq")]
    pub is_hq: bool,
    #[serde(rename = "onMannequin")]
    pub is_on_mannequin: bool,

    #[serde(rename = "quantity")]
    pub amount: u16,
    #[serde(rename = "pricePerUnit")]
    pub price_per_unit: u32,
    #[serde(rename = "total")]
    pub total_price: u32,

    #[serde(rename = "timestamp")]
    pub time_of_sale: u64,
    #[serde(rename = "buyerName")]
    pub buyer: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "event")]
pub enum Message {
    #[serde(rename = "listings/add")]
    ListingsAdd { world: u16, item: u16, listings: Vec<Listing> },
    #[serde(rename = "listings/remove")]
    ListingsRemove { world: u16, item: u16, listings: Vec<Listing> },
    #[serde(rename = "sales/add")]
    SalesAdd { world: u16, item: u16, sales: Vec<Sale> }, 
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to resolve the ip of 'universalis.app'.")]
    NameResolveFailed(std::io::Error),
    #[error("Failed to connect to 'universalis.app'")]
    ConnectFailed(),
    #[error("Websocket handshake failed")]
    WebsocketHandshakeFailed(tungstenite::HandshakeError<tungstenite::ClientHandshake<tungstenite::stream::MaybeTlsStream<TcpStream>>>),
    #[error("read_message failed")]
    ReadMessageFailed(tungstenite::Error),
}

pub struct UniversalisConnection {
    socket: tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<TcpStream>>,
}

pub fn connect() -> Result<UniversalisConnection, Error>{
    let connection = connect_to_universalis()?;
    _ = connection.set_read_timeout(Some(Duration::from_secs(15)));

    match tungstenite::client_tls("wss://universalis.app/api/ws", connection) {
        Ok(client) => Ok(UniversalisConnection { socket: client.0 }),
        Err(err) => Err(Error::WebsocketHandshakeFailed(err)),
    }
}

impl UniversalisConnection {
    pub fn subscribe(&mut self, channel: &str) {
        let bson = bson::rawdoc! {"event": "subscribe", "channel": channel };
        let message = tungstenite::Message::binary(bson.as_bytes());
        _ = self.socket.write_message(message);
    }

    pub fn unsubscribe(&mut self, channel: &str) {
        let bson = bson::rawdoc! {"event": "unsubscribe", "channel": channel };
        let message = tungstenite::Message::binary(bson.as_bytes());
        _ = self.socket.write_message(message);
    }

    pub fn read_message(&mut self) -> Result<Message, Error> {
        loop {
            let message = match self.socket.read_message() {
                Ok(message) => message,
                Err(tungstenite::Error::Io(err)) => {
                    if err.kind() == std::io::ErrorKind::TimedOut {
                        _ = self.socket.write_message(tungstenite::Message::Ping(vec![]));
                        continue
                    } else {
                        return Err(Error::ReadMessageFailed(tungstenite::Error::Io(err)));
                    }
                }
                Err(err) => return Err(Error::ReadMessageFailed(err)),
            };

            if !message.is_binary() {
                continue;
            }

            let message = message.into_data();
            //let raw_bson = bson::Document::from_reader(message.as_slice()).unwrap();
            //print!("{:?}", raw_bson);
            /*for listing in raw_bson.get_array("listings").unwrap() {
                for key in listing.as_document().unwrap().keys() {
                    print!("{:?}, ", key);
                }
                println!();
            }*/

            let message = match bson::from_slice::<Message>(&message) {
                Ok(message) => message,
                Err(_) => continue,
            };

            return Ok(message);
        }
    }
}

fn connect_to_universalis() -> Result<TcpStream, Error>{
    let addrs = ("universalis.app", 443).to_socket_addrs().map_err(|err| Error::NameResolveFailed(err))?;
    for addr in addrs {
        if let Ok(stream) = TcpStream::connect(addr) {
            _ = stream.set_nodelay(true);
            return Ok(stream);
        }
    }

    Err(Error::ConnectFailed())
}