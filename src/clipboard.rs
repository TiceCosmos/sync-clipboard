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
            clip: ClipboardContext::new().map_err(|e| Error::new_clip(e))?,
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
                    Error::Chn(_) => {
                        error!("{}", e);
                        break;
                    }
                    Error::Clp(_) => match ClipboardContext::new() {
                        Ok(clip) => self.clip = clip,
                        Err(e) => {
                            error!("Clipboard error: {}", e);
                            break;
                        }
                    },
                    _ => {}
                }
                warn!("{}", e);
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
                self.clip.set_contents(x).map_err(|e| Error::new_clip(e))?;
            }
            Err(x) => {
                let x = x.map_err(|e| Error::new_clip(e))?;
                if !x.starts_with("x-special/") {
                    let hash = crate::utils::calculate_hash(&x);
                    if self.last != hash {
                        self.last = hash;
                        if let Some(n) = crate::message::encode(self.buff.as_mut(), x) {
                            let message = self.buff[..n].as_ref();
                            let mut maps = self.maps.lock().await;
                            let mut task_list = vec![];
                            for stream in maps.values_mut() {
                                task_list.push(stream.write_all(message));
                            }
                            futures_util::future::join_all(task_list).await;
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
