use prost::Message;

pub trait ProtoImpl: Message + Sized {
    fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.encode(&mut buf).unwrap();
        buf
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, prost::DecodeError> 
    where Self: Default,
    {
        Self::decode(bytes)
    }
}

impl<T: Message + Sized> ProtoImpl for T {}

pub const KIND_TEST: &str = "TEST"; 
pub const KIND_HANDSHAKE: &str = "HANDSHAKE";
pub const KIND_PING: &str = "PING";
pub const KIND_PONG: &str = "PONG";
pub const KIND_PEER_INFO: &str = "PEER_INFO";

#[derive(Message)]
pub struct Base {
    #[prost(string, tag = 1)]
    pub kind: String,
    #[prost(bytes, tag = 2)]
    pub data: Vec<u8>,
}

#[derive(Message)]
pub struct Handshake {
    #[prost(uint32, tag = 1)]
    pub port: u32,
}

#[derive(Message)]
pub struct PeerInfo {
    #[prost(uint32, tag = 1)]
    pub port: u32,
    #[prost(string, tag = 2)]
    pub ip: String,
}