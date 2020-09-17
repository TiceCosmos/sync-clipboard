use crate::common::*;

use copypasta::{ClipboardContext, ClipboardProvider};
use log::warn;
use std::{
    error::Error,
    io::prelude::*,
    net::{IpAddr, SocketAddr, TcpStream},
    thread,
    time::Duration,
};

pub struct Client {
    addr: SocketAddr,
    send: [u8; BUF_LEN],
    recv: [u8; BUF_LEN * 2],
}

impl Client {
    pub fn bind<S: AsRef<str>>(addr: S, port: u16) -> Result<Self, Box<dyn Error>> {
        let addr = addr.as_ref().parse::<IpAddr>()?;
        Ok(Client {
            addr: (addr, port).into(),
            send: [0xff; BUF_LEN],
            recv: [0; BUF_LEN * 2],
        })
    }
    pub fn cycle(&mut self, open_url: bool) {
        let wait_time = Duration::from_millis(WAIT_MS);
        let reconnect = Duration::from_secs(5);
        loop {
            if let Err(e) = self.remote_synchronize(wait_time, open_url) {
                warn!("{}", e);
            }
            thread::sleep(reconnect);
        }
    }
    fn remote_synchronize(
        &mut self,
        wait_time: Duration,
        open_url: bool,
    ) -> Result<(), Box<dyn Error>> {
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
            recv_len = stream_recv(
                &mut stream,
                &mut self.recv,
                recv_len,
                open_url,
                |data: String| {
                    last_hash = calculate_hash(&data);
                    clipboard.set_contents(data).ok();
                    Ok(())
                },
            )?;
            thread::sleep(wait_time);
        }
    }
}
