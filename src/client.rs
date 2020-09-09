use crate::common::*;

use copypasta::{ClipboardContext, ClipboardProvider};
use log::warn;
use std::error::Error;
use std::io::prelude::*;
use std::net::{SocketAddr, TcpStream};
use std::thread;
use std::time::Duration;

pub struct Client {
    addr: SocketAddr,
    send: [u8; BUF_LEN],
    recv: [u8; BUF_LEN * 2],
}

impl Client {
    pub fn bind<S: AsRef<str>>(addr: S, port: u16) -> Result<Self, Box<dyn Error>> {
        Ok(Client {
            addr: (addr.as_ref().parse::<std::net::IpAddr>()?, port).into(),
            send: [0xff; BUF_LEN],
            recv: [0; BUF_LEN * 2],
        })
    }
    pub fn cycle(&mut self) {
        let wait_time = Duration::from_millis(WAIT_MS);
        loop {
            if let Err(e) = self.remote_synchronize(wait_time) {
                warn!("{}", e);
            }
        }
    }
    fn remote_synchronize(&mut self, wait_time: Duration) -> Result<(), Box<dyn Error>> {
        let mut stream = TcpStream::connect(self.addr)?;
        stream.set_read_timeout(Some(wait_time))?;
        let mut clipboard = ClipboardContext::new()?;

        let mut last_hash = calculate_hash(&String::new());

        let mut recv_len = 0;
        loop {
            clipboard_check(&mut clipboard, &mut last_hash, |data: String| {
                if let Some(n) = encode(&mut self.send, data) {
                    stream.write_all(&self.send[0..n])?;
                }
                Ok(())
            })?;
            recv_len = stream_recv(&mut stream, &mut self.recv, recv_len, |data: String| {
                last_hash = calculate_hash(&data);
                clipboard.set_contents(data).ok();
                Ok(())
            })?;
            thread::sleep(wait_time);
        }
    }
}
