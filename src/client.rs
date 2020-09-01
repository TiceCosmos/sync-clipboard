use crate::common::*;

use copypasta::{ClipboardContext, ClipboardProvider};
use log::warn;
use std::error::Error;
use std::io::prelude::*;
use std::net::{TcpStream, ToSocketAddrs};
use std::thread;
use std::time::Duration;

pub struct Client {
    stream: TcpStream,
    clipboard: ClipboardContext,
    wait_time: Duration,
}

impl Client {
    pub fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self, Box<dyn Error>> {
        let stream = TcpStream::connect(addr)?;
        let wait_time = Duration::from_millis(WAIT_MS);
        stream.set_read_timeout(Some(wait_time))?;
        let clipboard = ClipboardContext::new()?;
        Ok(Client {
            stream,
            clipboard,
            wait_time,
        })
    }
    pub fn cycle(&mut self) {
        if let Err(e) = self.remote_synchronize() {
            warn!("{}", e);
        }
    }
    fn remote_synchronize(&mut self) -> Result<(), Box<dyn Error>> {
        let stream = &mut self.stream;
        let clipboard = &mut self.clipboard;
        let wait_time = self.wait_time;

        let mut last_hash = calculate_hash(&String::new());

        let mut send_buf = [0xff; BUF_LEN];
        let mut recv_buf = [0; BUF_LEN * 2];
        let mut recv_len = 0;
        loop {
            clipboard_check(clipboard, &mut last_hash, |data: String| {
                if let Some(n) = encode(&mut send_buf, data) {
                    stream.write_all(&send_buf[0..n])?;
                }
                Ok(())
            })?;
            recv_len = stream_recv(stream, &mut recv_buf, recv_len, |data: String| {
                last_hash = calculate_hash(&data);
                clipboard.set_contents(data).ok();
                Ok(())
            })?;
            thread::sleep(wait_time);
        }
    }
}
