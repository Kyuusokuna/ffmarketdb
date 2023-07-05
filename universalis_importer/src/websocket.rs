use std::io::{Read, Write};
use serde::Deserialize;

#[derive(Deserialize)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Default)]
pub struct MateriaInfo {
    #[serde(rename = "slotID")]
    pub slot_index: u8,
    #[serde(rename = "materiaID")]
    pub materia_index: u8,
}

#[derive(Deserialize)]
#[derive(Debug, PartialEq, Eq, Clone, Default)]
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

#[derive(Deserialize)]
#[derive(Debug, PartialEq, Eq, Clone, Default)]
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

#[derive(Deserialize)]
#[derive(Debug, PartialEq, Eq, Clone)]
#[serde(tag = "event")]
pub enum Message {
    #[serde(rename = "listings/add")]
    ListingsAdd { world: u16, item: u16, listings: Vec<Listing> },
    #[serde(rename = "listings/remove")]
    ListingsRemove { world: u16, item: u16, listings: Vec<Listing> },
    #[serde(rename = "sales/add")]
    SalesAdd { world: u16, item: u16, sales: Vec<Sale> }, 
}

pub struct Connection<Stream: Read + Write> {
    inner: tungstenite::WebSocket<Stream>,
}

impl Connection<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>> {
    #[inline]
    pub fn connect() -> Result<Self, tungstenite::Error>{
        let connection = tungstenite::connect("wss://universalis.app/api/ws")?.0;
        _ = match connection.get_ref() {
            tungstenite::stream::MaybeTlsStream::Plain(stream) => stream.set_read_timeout(Some(core::time::Duration::from_secs(15))),
            tungstenite::stream::MaybeTlsStream::Rustls(stream) => stream.sock.set_read_timeout(Some(core::time::Duration::from_secs(15))),
            _ => Ok(()),
        };

        Ok(Self { inner: connection })
    }
}

impl<Stream: Read + Write> Connection<Stream> {
    #[inline]
    #[allow(dead_code)]
    pub fn subscribe(&mut self, channel: &str) {
        let bson = bson::rawdoc! {"event": "subscribe", "channel": channel };
        let message = tungstenite::Message::binary(bson.as_bytes());
        _ = self.inner.write_message(message);
    }

    #[inline]
    #[allow(dead_code)]
    pub fn unsubscribe(&mut self, channel: &str) {
        let bson = bson::rawdoc! {"event": "unsubscribe", "channel": channel };
        let message = tungstenite::Message::binary(bson.as_bytes());
        _ = self.inner.write_message(message);
    }

    #[inline]
    #[allow(dead_code)]
    pub fn read_message(&mut self) -> Result<Message, tungstenite::Error> {
        loop {
            let message = match self.inner.read_message() {
                Ok(message) => message,
                Err(tungstenite::Error::Io(err)) if err.kind() == std::io::ErrorKind::TimedOut => {
                    _ = self.inner.write_message(tungstenite::Message::Pong(vec![]));
                    continue
                }
                err => err?,
            };

            if !message.is_binary() {
                continue;
            }

            let Ok(message) = bson::from_slice::<Message>(&message.into_data()) else { continue };
            return Ok(message);
        }
    }
}