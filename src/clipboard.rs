use crate::{Error, PeerMap, Result, BUF_LEN, WAIT_MS};
use async_std::{channel::Receiver, future, prelude::*};
use copypasta::{ClipboardContext, ClipboardProvider};
use log::{error, warn};
use std::time::Duration;

pub struct Clipboard {
    clip: ClipboardContext,
    maps: PeerMap,
    rchn: Receiver<String>,
    last: u64,
    buff: Box<[u8; BUF_LEN]>,
}

impl Clipboard {
    pub fn new(rchn: Receiver<String>, maps: PeerMap) -> Result<Self, Error> {
        Ok(Self {
            clip: ClipboardContext::new()?,
            rchn,
            maps,
            last: crate::utils::calculate_hash(&String::new()),
            buff: Box::new([0; BUF_LEN]),
        })
    }
    pub async fn run(&mut self) {
        let wait_time = Duration::from_millis(WAIT_MS);

        loop {
            if let Err(e) = self.timeout_toggle(wait_time).await {
                match e {
                    Error::Lnk(e) => {
                        warn!("IO error: {}", e);
                    }
                    Error::Snd(e) => {
                        error!("Channel error: {}", e);
                        break;
                    }
                    Error::Rcv(e) => {
                        error!("Channel error: {}", e);
                        break;
                    }
                    Error::Dyn(e) => {
                        warn!("Clipboard get error: {}", e);
                    }
                }
            }
        }
    }
    async fn timeout_toggle(&mut self, wait_time: Duration) -> Result<()> {
        match future::timeout(wait_time, self.rchn.recv())
            .await
            .map_err(|_| self.clip.get_contents())
        {
            Ok(x) => {
                let x = x?;
                self.last = crate::utils::calculate_hash(&x);
                self.clip.set_contents(x)?;
            }
            Err(x) => {
                let x = x?;
                if !x.starts_with("x-special/") {
                    let hash = crate::utils::calculate_hash(&x);
                    if self.last != hash {
                        self.last = hash;
                        if let Some(n) = crate::message::encode(self.buff.as_mut(), x) {
                            let message = self.buff[..n].as_ref();
                            for stream in self.maps.lock().await.values_mut() {
                                stream.write_all(message).await?;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
