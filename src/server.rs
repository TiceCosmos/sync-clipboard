use crate::{client::Client, Error, PeerMap};
use async_std::{
    channel::Sender,
    io,
    net::{SocketAddr, TcpListener},
    prelude::*,
    task,
};
use log::{error, info, warn};

pub struct Server {
    listener: TcpListener,
    schn: Sender<String>,
    maps: PeerMap,
    auto: bool,
}

impl Server {
    pub async fn new(addr: SocketAddr, schn: Sender<String>, maps: PeerMap) -> io::Result<Self> {
        Ok(Server {
            listener: TcpListener::bind(addr).await?,
            schn,
            maps,
            auto: false,
        })
    }

    pub fn with_auto(&mut self) -> &mut Self {
        self.auto = true;
        self
    }

    pub async fn run(&mut self) {
        let mut incoming = self.listener.incoming();

        while let Some(stream) = incoming.next().await {
            let (stream, addr) = match stream.and_then(|s| s.local_addr().map(|a| (s, a))) {
                Ok(x) => x,
                Err(e) => {
                    warn!("{}", e);
                    continue;
                }
            };

            let mut client = Client::new(addr, self.schn.clone(), self.maps.clone());

            task::spawn(async move {
                info!("Client {:?} linked", addr);

                let (reader, writer) = futures_lite::io::split(stream);

                client.maps.lock().await.insert(addr, writer);

                if let Err(e) = client.recv_message(reader).await {
                    match e {
                        Error::Lnk(e) => {
                            warn!("IO error: {}", e);
                        }
                        Error::Snd(e) => {
                            error!("Channel error: {}", e);
                        }
                        Error::Rcv(e) => {
                            error!("Channel error: {}", e);
                        }
                        Error::Dyn(e) => {
                            warn!("Clipboard get error: {}", e);
                        }
                    }
                }

                client.maps.lock().await.remove(&addr);

                info!("Client {:?} unlink", addr);
            });
        }
    }
}
