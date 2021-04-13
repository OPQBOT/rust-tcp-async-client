#![allow(dead_code,unused)]
pub mod chatmsg;
pub mod client;
pub mod codec;
pub mod connections;
pub mod errors;
pub mod ping_request;
pub mod pong_response;
pub mod server;
pub mod stats;

use chatmsg::ChatMessage;
use nom::{alt, map, named, IResult};
use ping_request::PingRequest;
use pong_response::PongResponse;
use errors::PacketError;

pub trait FromBytes: Sized {
    /// Deserialize struct using `nom` from raw bytes
    fn from_bytes(i: &[u8]) -> IResult<&[u8], Self>;
}

/// The trait provides method to serialize struct into raw bytes
pub trait ToBytes: Sized {
    /// Serialize struct into raw bytes using `cookie_factory`
    //  fn to_bytes<'a>(&self, buf: (&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError>;
    fn to_bytes(&self) -> Result<Vec<u8>, PacketError>;
}
#[derive(Debug, PartialEq, Clone)]
pub enum Packet {
    /// [`Data`](./struct.Data.html) structure.
    //  Data(Vec<u8>),
    PingRequest(PingRequest),
    PongResponse(PongResponse),
    ChatMessage(ChatMessage),
    
}

impl FromBytes for Packet {
    named!(
        from_bytes<Packet>,
        alt!(
            map!(ChatMessage::from_bytes, Packet::ChatMessage)
                | map!(PongResponse::from_bytes, Packet::PongResponse)
                |map!(PingRequest::from_bytes, Packet::PingRequest)
        )
    );
}

impl ToBytes for Packet {
    //fn to_bytes<'a>(&self, buf: (&'a mut [u8], usize)) -> Result<(&'a mut [u8], usize), GenError> {
    fn to_bytes(&self) -> Result<Vec<u8>, PacketError> {
        match *self {
            Packet::PingRequest(ref p) => p.to_bytes(),
            Packet::PongResponse(ref p) => p.to_bytes(),
            Packet::ChatMessage(ref p) => p.to_bytes(),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
