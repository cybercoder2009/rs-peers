use tokio::sync::mpsc;
use crate::protocols::{
    ProtoImpl, Base, Handshake, 
    KIND_HANDSHAKE, KIND_PING, KIND_PONG,
};

#[derive(Debug, PartialEq, Clone)]
pub enum PeerDirection {
    UNKNOWN,
    INBOUND,
    OUTBOUND,
}

#[derive(Debug)]
pub struct Peer {
    pub port: u16,
    pub ts: u64,
    pub direction: PeerDirection,
    pub tx: Option<mpsc::Sender<Vec<u8>>>,
}

impl Peer {

    /// # send self listen ip and port
    /// - `port`: node binding port
    pub async fn send_handshake(
        &self,
        port: u16,
    ){
        let data = Handshake { port: port as u32}.to_bytes();
        let msg = Base {
            kind: KIND_HANDSHAKE.to_string(),
            data
        }.to_bytes();
        if self.tx.is_some() {
            let _ = self.tx.clone()
                .unwrap()
                .send(msg).await;
        }
    }

    pub async fn send_ping(
        &self,
    ){
        let msg = Base {
            kind: KIND_PING.to_string(),
            data: Vec::new(),
        }.to_bytes();
        if self.tx.is_some() {
            let _ = self.tx.clone()
                .unwrap()
                .send(msg).await;
        }
    }

    pub async fn send_pong(
        &self,
    ){
        let msg = Base {
            kind: KIND_PONG.to_string(),
            data: Vec::new(),
        }.to_bytes();
        if self.tx.is_some() {
            let _ = self.tx.clone()
                .unwrap()
                .send(msg).await;
        }
    }
}