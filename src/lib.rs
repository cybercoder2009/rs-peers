mod peer;
mod utils;
mod protocols;

use std::{
    sync::Arc,
    collections::HashMap
};
use tokio::{
    signal,
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, TcpListener},
    sync::{mpsc, RwLock},
    time::{Duration, sleep},
};
use peer::{Peer, PeerDirection};
use utils::{hhmmss, parse_ip_port, ts};
use protocols::{
    Base, Handshake, ProtoImpl, PeerInfo,
    KIND_TEST, KIND_HANDSHAKE, KIND_PING, KIND_PONG, KIND_PEER_INFO, 
};

pub struct Node {
    listener: TcpListener,
    ip: String,
    port: u16,
    peers: Arc<RwLock<HashMap<String, Peer>>>
}

impl Node {

    pub async fn init (
        opt_binding: Option<&str>,
        opt_seed: Option<&str>,
    ) -> Self {

        // listener
        let mut _binding = "0.0.0.0:0";
        if let Some(binding) = opt_binding {
            _binding = binding
        }
        let listener = TcpListener::bind(_binding)
        .await
        .unwrap_or_else(|err| 
            panic!("[node] init_listener err={}", err)
        );
        let addr = listener.local_addr()
        .unwrap_or_else(|err| 
            panic!("[node] init local_addr err={}", err)
        );
        let ip = addr.ip().to_string();
        let port = addr.port();
        
        // peers
        let mut inner = HashMap::new();
        if let Some(seed) = opt_seed {
            if let Some((ip, port)) = parse_ip_port(&seed) {
                inner.insert(
                    ip,
                    Peer {
                        port,
                        direction: PeerDirection::UNKNOWN,
                        tx: None,
                        ts: ts(),
                    }
                );
            }
        }
        let peers = Arc::new(RwLock::new(inner));
        Node {
            listener,
            ip,
            port,
            peers,
        }
        
    }

    async fn run_logging(
        &self,
        interval: u64,
    ) {
        loop {
            {
                let lock = self.peers.read().await;
                println!("\n=====================peers=====================");
                let mut total: u32 = 0;
                let mut active: u32 = 0;
                for (key, peer) in lock.iter() {
                    total += 1;
                    if peer.tx.is_some() { active += 1}
                    println!("[{}] port={} ts={} dir={:?} connected={}", 
                        key,
                        peer.port,
                        peer.ts,
                        peer.direction,
                        peer.tx.is_some(),
                    );
                }
                println!("=====================node======================");
                println!("binding({}:{}) peers({}/{})", &self.ip, self.port, active, total);
                println!("==================={}====================\n", hhmmss());
            }
            sleep(Duration::from_secs(interval)).await;
        }
    }

    async fn run_ping(
        &self,
        interval: u64,
    ) {
        loop {
            {
                let lock = self.peers.read().await;
                let opt_peer = lock.values()
                    .min_by_key(|peer| peer.ts);
                if let Some(peer) = opt_peer {
                    peer.send_ping().await;
                }
            }
            sleep(Duration::from_secs(interval)).await;
        }
    }

    /// outbound connections
    async fn run_outbound(
        &self,
        interval: u64,
    ) {
        loop {
            let target = {
                let reader = self.peers.read().await;
                reader.iter()
                .find_map(
                    |(ip, peer)| 
                    peer
                    .tx.is_none()
                    .then(|| (ip.clone(), peer.port))
                )
            };

            if let Some((ip, port)) = target {
                let peers = self.peers.clone();
                let node_port = self.port;
                tokio::spawn(async move {
                    let addr = format!("{}:{}", ip, port);
                    match TcpStream::connect(&addr).await {
                        Ok(stream) => {                      
                            Self::handle_outbound(
                                stream, 
                                peers,
                                node_port,
                            ).await
                        }
                        Err(err) => {
                            eprintln!("[node] run_outbound connect={}:{}  err={:?}", ip, port, err);
                        }
                    }
                });
            }
            sleep(Duration::from_secs(interval)).await;
        }
    }

    /// # inbound connections
    async fn run_listener(
        &self,
    ) {
        loop {
            match self.listener.accept().await {
                Ok((stream, _addr)) => {
                    let peers = self.peers.clone();
                    tokio::spawn(async move {
                        Self::handle_inbound(stream, peers).await;
                    });
                }
                Err(e) => {
                    eprintln!("[node] accept-error={}", e);
                }
            }
        } 
    }

    /// # handle inbound message
    /// - `buffer`: data
    /// - `ip`: source peer ip
    /// - `peers`
    pub async fn handle_message(
        buffer: &[u8],
        ip: String, 
        peers: Arc<RwLock<HashMap<String, Peer>>>
    ) {
        let base = Base::from_bytes(buffer).unwrap();
        let kind = base.kind.as_str();
        println!("[node] handle_message ip={} base={}", ip, base.kind);
        
        match kind {
            KIND_TEST => {
                sleep(Duration::from_millis(200)).await;
            },
            KIND_HANDSHAKE => {
                let handshake = Handshake::from_bytes(&base.data).unwrap();
                {
                    let mut lock = peers.write().await;
                    if let Some(peer) = lock.get_mut(&ip) {
                        peer.port = handshake.port as u16;
                    }
                }
                let ip = ip.clone();
                let port = handshake.port;
                let peers0 = peers.clone();
                tokio::spawn(async move {
                    let ip0 = ip.clone();
                    let base = Base {
                        kind: KIND_PEER_INFO.to_string(),
                        data: PeerInfo {ip, port}.to_bytes()
                    }.to_bytes();
                    let lock = peers0.read().await;
                    for (_key, peer) in 
                        lock.iter().filter(
                            |(key, peer)| 
                                peer.tx.is_some() && *key != &ip0
                        ) {
                        let tx = peer.tx.clone().unwrap();
                        let _ = tx.send(base.clone()).await;
                    }
                });
            },
            KIND_PEER_INFO => {
                let peer_info = PeerInfo::from_bytes(&base.data).unwrap();
                let mut lock = peers.write().await;
                lock.entry(peer_info.ip)
                    .or_insert(Peer {
                        port: peer_info.port as u16,
                        ts: ts(),
                        direction: PeerDirection::UNKNOWN,
                        tx: None,
                    });
            },
            KIND_PING => {
                let mut lock = peers.write().await;
                if let Some(peer) = lock.get_mut(&ip) {
                    peer.ts = ts();
                    peer.send_pong().await;
                }
            },
            KIND_PONG => {
                let mut lock = peers.write().await;
                if let Some(peer) = lock.get_mut(&ip) {
                    peer.ts = ts();
                }
            },
            _ => {
                eprintln!("[node] handle_message unknown-message={}", kind)
            }
        }
        
    }

    /// # parameters
    /// - `stream`: inbound stream
    /// - `peers`: maintained peers list
    pub async fn handle_inbound(
        stream: TcpStream,
        peers: Arc<RwLock<HashMap<String, Peer>>>
    ) {
        // info
        let addr = match stream.peer_addr() {
            Ok(addr) => addr,
            Err(_) => return,
        };
        let ip = addr.ip().to_string();
        let port = addr.port();
        println!("[node] inbound-connection={}:{}", ip, port);
        
        // connection split
        let (mut rx_stream , tx_stream) = stream.into_split();

        // write channel
        let (tx_mpsc, mut rx_mpsc) = mpsc::channel::<Vec<u8>>(32); 

        // flag if keep this stream
        let mut should_keep = false;
        
        // binding write channel before registration check
        let ip0 = ip.clone();
        tokio::spawn(async move {
            let mut tx_stream = tx_stream;
            while let Some(data) = rx_mpsc.recv().await {
                if let Err(e) = tx_stream.write_all(&data).await {
                    eprintln!("[node]  write-to-{} failed err={}", ip0, e);
                }
            }
            // rx_mpsc droppped here
        });

        // register
        {
            let mut lock = peers.write().await;
            if let Some(peer) = lock.get_mut(&ip) {
                if peer.tx.is_none() {
                    peer.tx = Some(tx_mpsc);
                    peer.direction = PeerDirection::INBOUND;
                    should_keep = true;
                } 
            } else {
                lock.insert(ip.clone(), Peer {
                    tx: Some(tx_mpsc),
                    port: 0,
                    direction: PeerDirection::INBOUND,
                    ts: ts(),
                });
                should_keep = true;
            }
        }
        
        // binding stream read channel
        if should_keep {
            let mut buf = [0; 1024];
            loop {
                match rx_stream.read(&mut buf).await {
                    Ok(0) => {
                        break
                    },
                    Ok(n) =>  {
                        println!("[node] inbound-msg from={} size={}", ip, n);
                        Self::handle_message(
                            &buf[..n],
                            ip.clone(),
                            peers.clone()
                        ).await;
                    },
                    Err(_) => break,
                }
            }
        }

        // clear on disconnect
        println!("[peer] {} disconnected", &ip);
        drop(rx_stream);
        peers.write().await.remove(&ip);
    }

    /// # parameters
    /// - `stream`: outbound stream
    /// - `peers`: maintained peers list
    /// - `node_port`: current node listen port
    pub async fn handle_outbound(
        stream: TcpStream,
        peers: Arc<RwLock<HashMap<String, Peer>>>,
        node_port: u16,
    ) {
        // info
        let addr = match stream.peer_addr() {
            Ok(addr) => addr,
            Err(_) => return,
        };
        let ip = addr.ip().to_string();
        let port = addr.port();
        println!("[node] outbound-connection={}:{}", ip, port);
        
        // connection split
        let (mut rx_stream , tx_stream) = stream.into_split();

        // write channel
        let (tx_mpsc, mut rx_mpsc) = mpsc::channel::<Vec<u8>>(32); 

        // flag if keep this stream
        let mut should_keep = false;
        
        // binding write channel before registration check
        let ip0 = ip.clone();
        tokio::spawn(async move {
            let mut tx_stream = tx_stream;
            while let Some(data) = rx_mpsc.recv().await {
                if let Err(e) = tx_stream.write_all(&data).await {
                    eprintln!("[node] write-to-{} failed err={}", ip0, e);
                }
            }
            // rx_mpsc dropped here 
        });

        // register
        {
            let mut lock = peers.write().await;
            if let Some(peer) = lock.get_mut(&ip) {
                if peer.tx.is_none() {
                    peer.tx = Some(tx_mpsc);
                    peer.direction = PeerDirection::OUTBOUND;
                    should_keep = true;
                    peer.send_handshake(
                        node_port
                    ).await;
                } 
            } 
        }
        
        // binding stream read channel
        if should_keep {
            let mut buf = [0; 1024];
            loop {
                match rx_stream.read(&mut buf).await {
                    Ok(0) => {
                        break
                    },
                    Ok(n) =>  {
                        println!("[node] inbound-msg from={} size={}", ip, n);
                        Self::handle_message(
                            &buf[..n],
                            ip.clone(),
                            peers.clone()
                        ).await;
                    },
                    Err(_) => break,
                }
            }
        }

        // clear on disconnect
        println!("[peer] {} disconnected", &ip);
        drop(rx_stream);
        peers.write().await.remove(&ip);
    }

    /// # parameters
    /// - `opt_logging`: logging interval in secs
    /// - `opt_pinging`: pinging interval in secs
    pub async fn run(
        self,
        opt_outbound: Option<&str>,
        opt_log: Option<&str>,
        opt_ping: Option<&str>,
    ) {
        let mut outbound: u64 = 5;
        if let Some(interval) = opt_outbound {
            outbound = interval.parse().unwrap();
        }

        let mut logging: u64 = 5;
        if let Some(interval) = opt_log {
            logging = interval.parse().unwrap();
        }

        let mut pinging: u64 = 300;
        if let Some(interval) = opt_ping {
            pinging = interval.parse().unwrap();
        }

        tokio::select! {
            _ = self.run_listener() => 
                println!("[node] listener exists"),
            _ = self.run_outbound(outbound) =>
                println!("[node] outbound exists"),
            _ = self.run_logging(logging) => 
                println!("[node] logging exists"),
            _ = self.run_ping(pinging) => 
                println!("[node] logging exists"),
            _ = signal::ctrl_c() => 
                println!("[node] ctrl_c exists"),
        }
    }
}