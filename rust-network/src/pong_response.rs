/*! PongResponse packet
*/

use crate::{FromBytes, ToBytes};
use bytes::BufMut;
use crate::errors::PacketError;
use nom::{do_parse, named, number::streaming::be_u64, tag};

/** Sent by both client and server, both will respond.
The server should respond to ping packets with pong packets with the same `ping_id`
as was in the ping packet. The server should check that each pong packet contains
the same `ping_id` as was in the ping, if not the pong packet must be ignored.
Serialized form:
Length | Content
------ | ------
`1`    | `0x05`
`8`    | ping_id in BigEndian
*/
#[derive(Debug, PartialEq, Clone)]
pub struct PongResponse {
    /// The id of ping to respond
    pub ping_id: u64,
}

impl FromBytes for PongResponse {
    named!(
        from_bytes<PongResponse>,
        do_parse!(tag!(b"\xbf") >> ping_id: be_u64 >> (PongResponse { ping_id }))
    );
}

impl ToBytes for PongResponse {
    fn to_bytes(&self) -> Result<Vec<u8>, PacketError> {
        let mut buf = Vec::<u8>::new();
        buf.put_u8(0xbf);
        buf.put_u64(123);
        Ok(buf.to_vec())
    }
}
