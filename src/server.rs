use crate::common::*;

use copypasta::{ClipboardContext, ClipboardProvider};
use log::{info, warn};
use std::error::Error;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::{thread, time};

static LINK_COUNT: AtomicUsize = AtomicUsize::new(0);

pub struct Server {
    listener: TcpListener,
    get_l_to_send: Arc<Mutex<Receiver<String>>>,
    recv_r_to_set: Sender<String>,
}

impl Server {
    pub fn bind<S: AsRef<str>>(addr: S, port: u16) -> Result<Self, Box<dyn Error>> {
        let listener = TcpListener::bind((addr.as_ref(), port))?;
        let (send_l, recv_l) = channel::<String>();
        let (send_r, recv_r) = channel::<String>();
        let clipboard = ClipboardContext::new()?;
        thread::spawn(move || {
            if let Err(e) = Self::monitor_clipboard(clipboard, send_l, recv_r) {
                warn!("{}", e);
            }
        });
        Ok(Server {
            listener,
            get_l_to_send: Arc::new(Mutex::new(recv_l)),
            recv_r_to_set: send_r,
        })
    }

    pub fn cycle(&self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    let l = self.get_l_to_send.clone();
                    let r = self.recv_r_to_set.clone();
                    thread::spawn(move || {
                        if let Ok(source) = stream.local_addr() {
                            info!("{:?} linked", source);
                            LINK_COUNT.fetch_add(1, Ordering::Relaxed);
                            if let Err(e) = Self::remote_synchronize(stream, l, r) {
                                warn!("{}", e);
                            }
                            LINK_COUNT.fetch_sub(1, Ordering::Relaxed);
                            info!("{:?} unlink", source);
                        }
                    });
                }
                Err(e) => warn!("{}", e),
            }
        }
    }

    fn monitor_clipboard(
        mut clipboard: ClipboardContext,
        get_l_to_send: Sender<String>,   // local clipboard context
        recv_r_to_set: Receiver<String>, // remote clipboard context
    ) -> Result<(), Box<dyn Error>> {
        let wait_time = time::Duration::from_millis(WAIT_MS);

        let mut last_hash = calculate_hash(&String::new());

        loop {
            clipboard_check(&mut clipboard, &mut last_hash, |data: String| {
                let len = LINK_COUNT.load(Ordering::Relaxed);
                for _ in 0..len {
                    get_l_to_send.send(data.clone())?;
                }
                Ok(())
            })?;
            if let Ok(data) = recv_r_to_set.recv_timeout(wait_time) {
                clipboard.set_contents(data).ok();
            }
            thread::sleep(wait_time);
        }
    }

    fn remote_synchronize(
        mut stream: TcpStream,
        get_l_to_send: Arc<Mutex<Receiver<String>>>, // local clipboard context
        recv_r_to_set: Sender<String>,               // remote clipboard context
    ) -> Result<(), Box<dyn Error>> {
        let wait_time = time::Duration::from_millis(WAIT_MS);
        stream.set_read_timeout(Some(wait_time))?;

        let mut send_buf = [0xff; BUF_LEN];
        let mut recv_buf = [0; BUF_LEN * 2];
        let mut recv_len = 0;
        let mut last_hash = calculate_hash(&String::new());

        loop {
            if let Ok(data) = if let Ok(recv) = get_l_to_send.lock() {
                recv.recv_timeout(wait_time)
            } else {
                break;
            } {
                if last_hash != calculate_hash(&data) {
                    if let Some(n) = encode(&mut send_buf, data) {
                        stream.write_all(&send_buf[0..n])?;
                    }
                }
            }
            recv_len = stream_recv(&mut stream, &mut recv_buf, recv_len, |data: String| {
                last_hash = calculate_hash(&data);
                recv_r_to_set.send(data)?;
                Ok(())
            })?;
            thread::sleep(wait_time);
        }
        Ok(())
    }
}
