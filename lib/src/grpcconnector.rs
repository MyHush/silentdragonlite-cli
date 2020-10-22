use log::{error};
use std::sync::Arc;
use zcash_primitives::transaction::{TxId};

use crate::grpc_client::{ChainSpec, BlockId, BlockRange, RawTransaction, CompactBlock,
                         TransparentAddressBlockFilter, TxFilter, Empty, LightdInfo, Coinsupply};
use tonic::transport::{Channel, ClientTlsConfig};
use tokio_rustls::{rustls::ClientConfig};
use tonic::{Request};

use threadpool::ThreadPool;
use std::sync::mpsc::channel;

use crate::PubCertificate;
use crate::grpc_client::compact_tx_streamer_client::CompactTxStreamerClient;

mod danger {
    use tokio_rustls::rustls;
    use webpki;

    pub struct NoCertificateVerification {}

    impl rustls::ServerCertVerifier for NoCertificateVerification {
        fn verify_server_cert(&self,
                              _roots: &rustls::RootCertStore,
                              _presented_certs: &[rustls::Certificate],
                              _dns_name: webpki::DNSNameRef<'_>,
                              _ocsp: &[u8]) -> Result<rustls::ServerCertVerified, rustls::TLSError> {
            Ok(rustls::ServerCertVerified::assertion())
        }
    }
}

async fn get_client(uri: &http::Uri, no_cert: bool) -> Result<CompactTxStreamerClient<Channel>, Box<dyn std::error::Error>> {
    let channel = if uri.scheme_str() == Some("http") {
        //println!("http");
        Channel::builder(uri.clone()).connect().await?
    } else {
        //println!("https");
        let mut config = ClientConfig::new();

        config.alpn_protocols.push(b"h2".to_vec());
        config.root_store.add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        config.root_store.add_pem_file(
                &mut PubCertificate::get("lightwalletd-lite.myhush.pem").unwrap().as_ref()).unwrap();
        
        if no_cert {
            config.dangerous()
                .set_certificate_verifier(Arc::new(danger::NoCertificateVerification {}));
        }

        let tls = ClientTlsConfig::new()
            .rustls_client_config(config)
            .domain_name("lite.myhush.org");
        
        Channel::builder(uri.clone())
            .tls_config(tls)
            .connect()
            .await?
    };

    Ok(CompactTxStreamerClient::new(channel))
}

// ==============
// GRPC code
// ==============
async fn get_lightd_info(uri: &http::Uri, no_cert: bool) -> Result<LightdInfo, Box<dyn std::error::Error>> {
    let mut client = get_client(uri, no_cert).await?;

    let request = Request::new(Empty {});

    let response = client.get_lightd_info(request).await?;

    Ok(response.into_inner())
}

pub fn get_info(uri: &http::Uri, no_cert: bool) -> Result<LightdInfo, String> {
    let mut rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;

    rt.block_on(get_lightd_info(uri, no_cert)).map_err( |e| e.to_string())
}

 
async fn get_coinsupply_info(uri: &http::Uri, no_cert: bool) -> Result<Coinsupply, Box<dyn std::error::Error>> {
    let mut client = get_client(uri, no_cert).await?;

    let request = Request::new(Empty {});

    let response = client.get_coinsupply(request).await?;

    Ok(response.into_inner())
}
pub fn get_coinsupply(uri: http::Uri, no_cert: bool) -> Result<Coinsupply, String> {
    let mut rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
   
    rt.block_on(get_coinsupply_info(&uri, no_cert)).map_err( |e| e.to_string())
   // tokio::runtime::current_thread::Runtime::new().unwrap().block_on(runner)
}

async fn get_block_range<F : 'static + std::marker::Send>(uri: &http::Uri, start_height: u64, end_height: u64, no_cert: bool, pool: ThreadPool, c: F) 
    -> Result<(), Box<dyn std::error::Error>> 
where F : Fn(&[u8], u64) {
    let mut client = get_client(uri, no_cert).await?;

    let bs = BlockId{ height: start_height, hash: vec!()};
    let be = BlockId{ height: end_height,   hash: vec!()};

    let request = Request::new(BlockRange{ start: Some(bs), end: Some(be) });

     // Channel where the blocks are sent. A None signifies end of all blocks
     let (tx, rx) = channel::<Option<CompactBlock>>();

     // Channel that the processor signals it is done, so the method can return
     let (ftx, frx) = channel();
 
     // The processor runs on a different thread, so that the network calls don't
     // block on this
     pool.execute(move || {
         while let Some(block) = rx.recv().unwrap() {
             use prost::Message;
             let mut encoded_buf = vec![];
 
             block.encode(&mut encoded_buf).unwrap();
             c(&encoded_buf, block.height);
         }
 
         ftx.send(Ok(())).unwrap();
     });

    let mut response = client.get_block_range(request).await?.into_inner();
    //println!("{:?}", response);
    while let Some(block) = response.message().await? {
        tx.send(Some(block)).unwrap();
    }
    tx.send(None).unwrap();

    // Wait for the processor to exit
    frx.iter().take(1).collect::<Result<Vec<()>, String>>()?;

    Ok(())
}

pub fn fetch_blocks<F : 'static + std::marker::Send>(uri: &http::Uri, start_height: u64, end_height: u64, no_cert: bool, pool: ThreadPool, c: F) -> Result<(), String>
    where F : Fn(&[u8], u64) {
    
    let mut rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let es = format!("Error creating runtime {:?}", e);
            error!("{}", es);
            eprintln!("{}", e);
            return Err(es);
        }
    };

    match rt.block_on(get_block_range(uri, start_height, end_height, no_cert, pool, c)) {
        Ok(o) => Ok(o),
        Err(e) => {
            let e = format!("Error fetching blocks {:?}", e);
            error!("{}", e);
            eprintln!("{}", e);
            Err(e)
        }
    }
}


// get_address_txids GRPC call
async fn get_address_txids<F : 'static + std::marker::Send>(uri: &http::Uri, address: String, 
        start_height: u64, end_height: u64, no_cert: bool, c: F) -> Result<(), Box<dyn std::error::Error>>
    where F : Fn(&[u8], u64) {

    let mut client = get_client(uri, no_cert).await?;
    let start = Some(BlockId{ height: start_height, hash: vec!()});
    let end   = Some(BlockId{ height: end_height,   hash: vec!()});

    let request = Request::new(TransparentAddressBlockFilter{ address, range: Some(BlockRange{start, end}) });

    let maybe_response = client.get_address_txids(request).await?;
    let mut response = maybe_response.into_inner();

    while let Some(tx) = response.message().await? {
        c(&tx.data, tx.height);
    }

    Ok(())
}


pub fn fetch_transparent_txids<F : 'static + std::marker::Send>(uri: &http::Uri, address: String, 
        start_height: u64, end_height: u64, no_cert: bool, c: F) -> Result<(), String>
    where F : Fn(&[u8], u64) {
    
    let mut rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let e = format!("Error creating runtime {:?}", e);
            error!("{}", e);
            eprintln!("{}", e);
            return Err(e);
        }
    };

    match rt.block_on(get_address_txids(uri, address.clone(), start_height, end_height, no_cert, c)) {
        Ok(o) => Ok(o),
        Err(e) => {
            let e = format!("Error with get_address_txids runtime {:?}", e);
            error!("{}", e);
            eprintln!("{}", e);
            Err(e)
        }
    }
}


// get_transaction GRPC call
async fn get_transaction(uri: &http::Uri, txid: TxId, no_cert: bool) 
    -> Result<RawTransaction, Box<dyn std::error::Error>> {
    let mut client = get_client(uri, no_cert).await?;
    let request = Request::new(TxFilter { block: None, index: 0, hash: txid.0.to_vec() });

    let response = client.get_transaction(request).await?;

    Ok(response.into_inner())
}

pub fn fetch_full_tx(uri: &http::Uri, txid: TxId, no_cert: bool) -> Result<Vec<u8>, String> {
    let mut rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let errstr = format!("Error creating runtime {}", e.to_string());
            error!("{}", errstr);
            eprintln!("{}", errstr);
            return Err(errstr);
        }
    };

    match rt.block_on(get_transaction(uri, txid, no_cert)) {
        Ok(rawtx) => Ok(rawtx.data.to_vec()),
        Err(e) => {
            let errstr = format!("Error in get_transaction runtime {}", e.to_string());
            error!("{}", errstr);
            eprintln!("{}", errstr);
            Err(errstr)
        }
    }    
}

// send_transaction GRPC call
async fn send_transaction(uri: &http::Uri, no_cert: bool, tx_bytes: Box<[u8]>) -> Result<String, Box<dyn std::error::Error>> {
    let mut client = get_client(uri, no_cert).await?;

    let request = Request::new(RawTransaction {data: tx_bytes.to_vec(), height: 0});

    let response = client.send_transaction(request).await?;

    let sendresponse = response.into_inner();
    if sendresponse.error_code == 0 {
        let mut txid = sendresponse.error_message;
        if txid.starts_with("\"") && txid.ends_with("\"") {
            txid = txid[1..txid.len()-1].to_string();
        }

        Ok(txid)
    } else {
        Err(Box::from(format!("Error: {:?}", sendresponse)))
    }
}

pub fn broadcast_raw_tx(uri: &http::Uri, no_cert: bool, tx_bytes: Box<[u8]>) -> Result<String, String> {
    let mut rt = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;

    rt.block_on(send_transaction(uri, no_cert, tx_bytes)).map_err( |e| e.to_string())
}

// get_latest_block GRPC call
async fn get_latest_block(uri: &http::Uri, no_cert: bool) -> Result<BlockId, Box<dyn std::error::Error>> {
    let mut client = get_client(uri, no_cert).await?;

    let request = Request::new(ChainSpec {});

    let response = client.get_latest_block(request).await?;

    Ok(response.into_inner())
}

pub fn fetch_latest_block(uri: &http::Uri, no_cert: bool) -> Result<BlockId, String> {
    let mut rt = match tokio::runtime::Runtime::new() {
        Ok(r) => r,
        Err(e) => {
            let errstr = format!("Error creating runtime {}", e.to_string());
            eprintln!("{}", errstr);
            return Err(errstr);
        }
    };

    rt.block_on(get_latest_block(uri, no_cert)).map_err(|e| {
        let errstr = format!("Error getting latest block {}", e.to_string());
        eprintln!("{}", errstr);
        errstr
    })
}
