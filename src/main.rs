use std::collections::HashMap;
use node::Node;

fn parse_args() -> HashMap<String, String> {
    let mut args = HashMap::new();
    for arg in std::env::args().skip(1) {
        if let Some((key, value)) = arg.split_once('=') {
            args.insert(key.to_string(), value.to_string());
        } else {
            eprintln!("Warning: Invalid argument format '{}', expected key=value", arg);
        }
    }
    args
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    
    // args
    let args = parse_args();
    let opt_binding = args.get("binding").map(|s| s.as_str());
    let opt_seed = args.get("seed").map(|s| s.as_str());
    let opt_outbound = args.get("outbound").map(|s| s.as_str());
    let opt_logging = args.get("logging").map(|s| s.as_str());
    let opt_pinging = args.get("pinging").map(|s| s.as_str());
    
    // node
    let node = Node::init(
        opt_binding,
        opt_seed,
    ).await;
    node.run(
        opt_outbound,
        opt_logging,
        opt_pinging,   
    ).await;

    // // init
    // let (tx, mut rx) = mpsc::channel::<Inbound>(1024);

    

    // // inbound
    // let peers_in = peers.clone();
    // while let Ok((mut stream, addr)) = listener.accept().await {
    //     let peers_in_0 = peers_in.clone();
    //     tokio::spawn(async move {

    //         let (mut reader, mut writer) = stream.split();
    //         let mut buf = [0; 1024];



    //         let ip = addr.ip().to_string();
    //         let mut writer = peers_in_0.write().await;
            
    //         match writer.get_mut(&ip) {
    //             Some(peer) => {
    //                 if peer.stream.is_none() {
    //                     peer.stream = Some(stream);
    //                 } else {
    //                     let _ = stream.flush().await;
    //                     let _ = stream.shutdown().await;
    //                 }
    //             }
    //             None => {
    //                 writer.insert(ip, Peer {
    //                     port: addr.port(),
    //                     stream: Some(stream),
    //                     failures: 5,
    //                 });
    //             }
    //         }
    //     });
    // }

    Ok(())
}