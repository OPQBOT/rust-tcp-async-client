/*! Codec implementation for encoding/decoding TCP Packets in terms of tokio-io
*/

use std::io::Error as IoError;

use crate::{stats::Stats, FromBytes, Packet, ToBytes};
use bytes::{ BytesMut};
use crate::errors::{PacketError};
use failure::Fail;
use nom::{error::ErrorKind, Err};
use tokio_util::codec::{Decoder, Encoder};
//https://github.com/lucis-fluxum/utp-rs/blob/1be2589d924ac2053a6f31f20e134bbb77545b69/src/packet.rs
/// Error that can happen when decoding `Packet` from bytes
#[derive(Debug, Fail)]
pub enum DecodeError {
    /// Error indicates that received encrypted packet can't be parsed
    #[fail(
        display = "Deserialize EncryptedPacket error: {:?}, buffer: {:?}",
        error, buf
    )]
    DeserializeEncryptedError {
        /// Parsing error
        error: ErrorKind,
        /// TCP buffer
        buf: Vec<u8>,
    },
    /// Error indicates that received encrypted packet can't be decrypted
    #[fail(display = "Decrypt EncryptedPacket error")]
    DecryptError,
    /// Error indicates that more data is needed to parse decrypted packet
    #[fail(
        display = "Decrypted packet should not be incomplete, packet: {:?}",
        packet
    )]
    IncompleteDecryptedPacket {
        /// Received packet
        packet: Vec<u8>,
    },
    /// Error indicates that decrypted packet can't be parsed
    // #[fail(
    //     display = "Deserialize decrypted packet error: {:?}, packet: {:?}",
    //     error, packet
    // )]
    // DeserializeDecryptedError {
    //     /// Parsing error
    //     error: ErrorKind,
    //     /// Received packet
    //     packet: Vec<u8>,
    // },
    /// General IO error
    #[fail(display = "IO error: {:?}", error)]
    IoError {
        /// IO error
        #[fail(cause)]
        error: IoError,
    },
}

impl From<IoError> for DecodeError {
    fn from(error: IoError) -> DecodeError {
        DecodeError::IoError { error }
    }
}

/// Error that can happen when encoding `Packet` to bytes
#[derive(Debug, Fail)]
pub enum EncodeError {
    /// Error indicates that `Packet` is invalid and can't be serialized
    #[fail(display = "Serialize Packet error: {:?}", error)]
    SerializeError {
        /// Serialization error
        error: PacketError,
    },
    /// General IO error
    #[fail(display = "IO error: {:?}", error)]
    IoError {
        /// IO error
        #[fail(cause)]
        error: IoError,
    },
}

impl From<IoError> for EncodeError {
    fn from(error: IoError) -> EncodeError {
        EncodeError::IoError { error }
    }
}

/// implements tokio-io's Decoder and Encoder to deal with Packet
pub struct Codec {
    stats: Stats,
}

impl Codec {
    /// create a new Codec with the given Channel
    pub fn new(stats: Stats) -> Codec {
        Codec { stats }
    }
}

impl Decoder for Codec {
    type Item = Packet;
    type Error = DecodeError;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // deserialize EncryptedPacket

        // buf.advance(1);
        println!("bufs {:#?}", hex::encode(&buf));
        // deserialize Packet

        if buf.is_empty() {
            
            return Ok(None);
        }
        match Packet::from_bytes(&buf) {
            Err(Err::Incomplete(_needed)) => {
                // println!("{:#?}", needed);
                Err(DecodeError::IncompleteDecryptedPacket {
                    packet: buf.to_vec(),
                })
            }
            Err(Err::Error(_)) => {
                // let (_, kind) = error;
                //return Err(DecodeError::DeserializeEncryptedError { error: kind, buf: buf.to_vec() })
                //println!("1");
                return Ok(None);
            }
            Err(Err::Failure(_)) => {
                // let (_, kind) = error;
                //return Err(DecodeError::DeserializeEncryptedError { error: kind, buf: buf.to_vec() })
                // println!("2");
                return Ok(None);
            }
            Ok((_i, packet)) => {
                //println!("buf len {}", buf.len());
                // Add 1 to incoming counter
                self.stats.counters.increase_incoming();

                buf.clear();
                Ok(Some(packet))
            }
        }
    }
}

impl Encoder<Packet> for Codec {
    type Error = EncodeError;

    fn encode(&mut self, packet: Packet, buf: &mut BytesMut) -> Result<(), Self::Error> {
        // Add 1 to outgoing counter
        self.stats.counters.increase_outgoing();

        // serialize Packet
        // let mut packet_buf = [0; 2032];

        //let data:Vec<u8>=vec![1,2,3,4,5,66,7];
        let bufs = packet
            .to_bytes()
            .map_err(|error| EncodeError::SerializeError { error })?;
        println!("send {:#?}", hex::encode(&bufs));
        buf.extend_from_slice(&bufs);
        Ok(())
    }
}
