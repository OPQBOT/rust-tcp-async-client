/*! ChatMessage packet
*/

use crate::errors::PacketError;
use bytes::BufMut;
use mlua::{MetaMethod, ToLua, UserData, UserDataMethods};
use nom::{combinator::rest, do_parse, map_res, named, number::streaming::be_u64, tag, take};

use crate::{FromBytes, ToBytes};
/** Sent by both client and server, both will respond.
Ping packets are used to know if the other side of the connection is still
live. TCP when established doesn't have any sane timeouts (1 week isn't sane)
so we are obliged to have our own way to check if the other side is still live.
Ping ids can be anything except 0, this is because of how toxcore sets the
variable storing the `ping_id` that was sent to 0 when it receives a pong
response which means 0 is invalid.
The server should send ping packets every X seconds (toxcore `TCP_server` sends
them every 30 seconds and times out the peer if it doesn't get a response in 10).
The server should respond immediately to ping packets with pong packets.
Serialized form:
Length | Content
------ | ------
`1`    | `0x04`
`8`    | ping_id in BigEndian
*/
#[derive(Debug, PartialEq, Clone)]
pub struct ChatMessage {
    pub msg_id: u64,
    pub to_user: String,
    pub from_user: String,
    pub content: Vec<u8>,
}

impl FromBytes for ChatMessage {
    named!(
        from_bytes<ChatMessage>,
        do_parse!(
            tag!("\x04")
                >> msg_id: be_u64
                >> len: be_u64
                >> to_user: map_res!(take!(len as usize), std::str::from_utf8)
                >> len: be_u64
                >> from_user: map_res!(take!(len as usize), std::str::from_utf8)
                >> content: rest
                >> (ChatMessage {
                    msg_id,
                    to_user: to_user.to_string(),
                    from_user: from_user.to_string(),
                    content: content.to_vec()
                })
        )
    );
}

impl ToBytes for ChatMessage {
    fn to_bytes(&self) -> Result<Vec<u8>, PacketError> {
        let mut buf = Vec::<u8>::new();
        buf.put_u8(0x4);
        buf.put_u64(self.msg_id);
        buf.put_u64(self.to_user.as_bytes().len() as u64);
        buf.extend_from_slice(self.to_user.as_bytes());
        buf.put_u64(self.from_user.as_bytes().len() as u64);
        buf.extend_from_slice(self.from_user.as_bytes());
        buf.extend_from_slice(&self.content);
        Ok(buf.to_vec())
    }
}

impl UserData for ChatMessage {
    fn add_methods<'lua, M: UserDataMethods<'lua, Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::Index, |ctx, this: &ChatMessage, arg: String| {
            let r = match arg.as_str() {
                "to_user" => this.to_user.as_str().to_lua(ctx).ok(),
                "from_user" => this.from_user.as_str().to_lua(ctx).ok(),
                "msg_id" => this.msg_id.to_lua(ctx).ok(),
                _ => {
                    println!("arg {}", arg);
                    None
                }
            };

            Ok(r)
        });
    }
}
