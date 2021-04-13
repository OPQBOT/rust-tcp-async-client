//! `error_kind` macros that helps to construct errors using Error-ErrorKind
//! pair pattern.

/// Helps to construct errors using Error-ErrorKind pair pattern.
macro_rules! error_kind {
    ($(#[$error_attr:meta])* $error:ident, $(#[$kind_attr:meta])* $kind:ident { $($variants:tt)* }) => {
        $(#[$error_attr])*
        pub struct $error {
            ctx: failure::Context<$kind>,
        }

        impl $error {
            /// Return the kind of this error.
            #[allow(dead_code)] // might be unused if error is private
            pub fn kind(&self) -> &$kind {
                self.ctx.get_context()
            }
        }

        impl failure::Fail for $error {
            fn cause(&self) -> Option<&dyn failure::Fail> {
                self.ctx.cause()
            }

            fn backtrace(&self) -> Option<&failure::Backtrace> {
                self.ctx.backtrace()
            }
        }

        impl std::fmt::Display for $error {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                self.ctx.fmt(f)
            }
        }

        $(#[$kind_attr])*
        pub enum $kind {
            $($variants)*
        }

        impl From<$kind> for $error {
            fn from(kind: $kind) -> $error {
                $error::from(failure::Context::new(kind))
            }
        }

        impl From<failure::Context<$kind>> for $error {
            fn from(ctx: failure::Context<$kind>) -> $error {
                $error { ctx }
            }
        }
    }
}

error_kind! {
    #[doc = "Error that can happen when handling `Tcp relay` packet."]
    #[derive(Debug)]
    HandlePacketError,
    #[doc = "The specific kind of error that can occur."]
    #[derive(Clone, Debug, Eq, PartialEq, failure::Fail)]
    HandlePacketErrorKind {
        #[doc = "Send packet(s) error."]
        #[fail(display = "Send packet(s) error")]
        SendTo,
        #[doc = "Server must not send this packet to client."]
        #[fail(display = "Server must not send this packet to client")]
        MustNotSend,
        #[doc = "Invalid connection ID when handling RouteResponse."]
        #[fail(display = "Invalid connection ID when handling RouteResponse")]
        InvalidConnectionId,
        #[doc = "Connection ID is already linked."]
        #[fail(display = "Connection ID is already linked")]
        AlreadyLinked,
        #[doc = "Unexpected route response packet is received."]
        #[fail(display = "Unexpected route response packet is received")]
        UnexpectedRouteResponse,
  }
}

error_kind! {
    #[doc = "Error that can happen when sending packet."]
    #[derive(Debug)]
    SendPacketError,
    #[doc = "The specific kind of error that can occur."]
    #[derive(Clone, Debug, Eq, PartialEq, failure::Fail)]
    SendPacketErrorKind {
        #[doc = "Send packet(s) error."]
        #[fail(display = "Send packet(s) error")]
        SendTo,
        #[doc = "Send packet(s) with wrong status."]
        #[fail(display = "Send packet(s) with wrong status")]
        WrongStatus,
        #[doc = "Send packet(s) with destination_pk is not online."]
        #[fail(display = "Send packet(s) with destination_pk is not online")]
        NotOnline,
        #[doc = "Send packet(s) with destination_pk is not linked."]
        #[fail(display = "Send packet(s) with destination_pk is not linked")]
        NotLinked,
        #[doc = "Send packet(s) to a connection but no such connection."]
        #[fail(display = "Send packet(s) to a connection but no such connection")]
        NoSuchConnection,
        #[doc = "Send packet(s) to a connection TimeOut."]
        #[fail(display = "Send packet(s) to a TimeOut")]
        TimeOut,
    }
}

error_kind! {
    #[doc = "Error that can happen when spawning a connection."]
    #[derive(Debug)]
    SpawnError,
    #[doc = "The specific kind of error that can occur."]
    #[derive(Clone, Debug, Eq, PartialEq, failure::Fail)]
    SpawnErrorKind {
        #[doc = "Read socket to receive packet error."]
        #[fail(display = "Read socket to receive packet error")]
        ReadSocket,
        #[doc = "Send packet(s) error."]
        #[fail(display = "Send packet(s) error")]
        SendTo,
        #[doc = "Handle packet(s) error."]
        #[fail(display = "Handle packet(s) error")]
        HandlePacket,
        #[doc = "Tcp client io error."]
        #[fail(display = "Tcp client io error")]
        Io,
        #[doc = "Tcp codec encode error."]
        #[fail(display = "Tcp codec encode error")]
        Encode,
    }
}

error_kind! {
    #[derive(Debug)]
    PacketError,
    #[derive(Clone, Debug, Eq, PartialEq, failure::Fail)]
    PacketErrorKind {
        #[fail(display = "to_bytes err")]
        PackErr,
        #[fail(display = "from_bytes err")]
        UnPackErr,
    }
}

error_kind! {
    #[doc = "Error that can happen when handling a connection."]
    #[derive(Debug)]
    ConnectionError,
    #[doc = "The specific kind of error that can occur."]
    #[derive(Clone, Debug, Eq, PartialEq, failure::Fail)]
    ConnectionErrorKind {
        #[doc = "Spawing after adding global connection error."]
        #[fail(display = "Spawing after adding global connection error")]
        Spawn,
        #[doc = "Search relay by relay's PK, but no such relay."]
        #[fail(display = "Search relay by relay's PK, but no such relay")]
        NoSuchRelay,
        #[doc = "Send packet(s) error."]
        #[fail(display = "Send packet(s) error")]
        SendTo,
        #[doc = "No connection to the node."]
        #[fail(display = "No connection to the node")]
        NoConnection,
        #[doc = "Relay is not connected."]
        #[fail(display = "Relay is not connected")]
        NotConnected,
        #[doc = "Tcp Connections wakeup timer error."]
        #[fail(display = "Tcp Connections wakeup timer error")]
        Wakeup,
        #[doc = "Add connection to client error."]
        #[fail(display = "Add connection to client error")]
        AddConnection,
    }
}

#[cfg(test)]
mod tests {
    use failure::Fail;

    error_kind! {
        #[derive(Debug)]
        TestError,
        #[derive(Clone, Debug, Eq, PartialEq, Fail)]
        TestErrorKind {
            #[fail(display = "Variant1")]
            Variant1,
            #[fail(display = "Variant2")]
            Variant2,
        }
    }

    #[test]
    fn test_error() {
        assert_eq!(
            format!("{}", TestErrorKind::Variant1),
            "Variant1".to_owned()
        );
        assert_eq!(
            format!("{}", TestErrorKind::Variant2),
            "Variant2".to_owned()
        );
    }

    #[test]
    fn test_error_variant_1() {
        let error = TestError::from(TestErrorKind::Variant1);
        assert_eq!(error.kind(), &TestErrorKind::Variant1);
        assert!(error.cause().is_none());
        assert!(error.backtrace().is_some());
        assert_eq!(format!("{}", error), "Variant1".to_owned());
    }

    #[test]
    fn test_error_variant_2() {
        let error = TestError::from(TestErrorKind::Variant2);
        assert_eq!(error.kind(), &TestErrorKind::Variant2);
        assert!(error.cause().is_none());
        assert!(error.backtrace().is_some());
        assert_eq!(format!("{}", error), "Variant2".to_owned());
    }
}
