use crate::{Error, PeerMap, Result, BUF_LEN, RECON_S};
use async_std::{
    channel::Sender,
    net::{SocketAddr, TcpStream},
    prelude::*,
    task,
};
use futures_lite::io::ReadHalf;
use log::{error, info, warn};
use std::time::Duration;

pub struct Client {
    pub addr: SocketAddr,
    pub maps: PeerMap,
    schn: Sender<String>,
    buff: Box<[u8; BUF_LEN * 2]>,
    auto: bool,
}

impl Client {
    pub fn new(addr: SocketAddr, schn: Sender<String>, maps: PeerMap) -> Self {
        Client {
            addr,
            schn,
            maps,
            buff: Box::new([0; BUF_LEN * 2]),
            auto: false,
        }
    }

    pub fn with_auto(&mut self) -> &mut Self {
        self.auto = true;
        self
    }

    pub async fn run(&mut self) {
        let reconnect = Duration::from_secs(RECON_S);
        loop {
            let stream = match TcpStream::connect(self.addr).await {
                Ok(x) => x,
                Err(e) => {
                    warn!("Tcp connect error: {}", e);
                    task::sleep(reconnect).await;
                    continue;
                }
            };

            let (reader, writer) = futures_lite::io::split(stream);

            self.maps.lock().await.insert(self.addr, writer);

            if let Err(e) = self.recv_message(reader).await {
                match e {
                    Error::Chn(_) => {
                        error!("{}", e);
                        break;
                    }
                    _ => {
                        warn!("{}", e);
                    }
                }
            }

            self.maps.lock().await.remove(&self.addr);
        }
    }

    pub async fn recv_message(&mut self, mut reader: ReadHalf<TcpStream>) -> Result<()> {
        let mut has_len = 0;

        loop {
            let new_len = reader.read(&mut self.buff[has_len..]).await?;
            if new_len == 0 {
                return Ok(());
            }
            info!("收到数据长度: {}", new_len);
            has_len += new_len;
            loop {
                match crate::message::decode(&mut self.buff[..has_len]) {
                    Ok(Some((len, data))) => {
                        if self.auto
                            && (data.starts_with("http://") || data.starts_with("https://"))
                        {
                            webbrowser::open(&data).ok();
                        }
                        self.schn.send(data).await?;
                        has_len = len;
                    }
                    Ok(None) => break,
                    Err(_) => break,
                }
            }
        }
    }
}
