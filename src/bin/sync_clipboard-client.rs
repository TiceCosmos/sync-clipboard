use copypasta::{ClipboardContext, ClipboardProvider};
use log::{info, warn};
use std::error::Error;
use std::io::prelude::*;
use std::net::TcpStream;
use std::{thread, time};
use structopt::StructOpt;
use sync_clipboard::*;

#[derive(StructOpt, Debug)]
struct Opt {
    /// server addr
    #[structopt(short = "a", long = "addr", default_value = "127.0.0.1")]
    addr: String,
    /// server port
    #[structopt(short = "p", long = "port", default_value = "43190")]
    port: u16,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger_init();
    let opt = Opt::from_args();
    info!("{:#?}", opt);

    let mut stream = TcpStream::connect((opt.addr, opt.port))?;
    let wait_time = time::Duration::from_millis(WAIT_MS);
    stream.set_read_timeout(Some(wait_time))?;

    let mut clip = ClipboardContext::new()?;

    let mut last_hash = calculate_hash(&String::new());

    let mut send_buf = [0xff; 1024];
    let mut recv_buf = [0; 2048];
    let mut recv_len = 0;
    loop {
        if let Ok(data) = clip.get_contents() {
            let curr_hash = calculate_hash(&data);
            if last_hash != curr_hash {
                if let Some(n) = encode(&mut send_buf, data) {
                    if stream.write(&send_buf[0..n]).is_err() {
                        break;
                    }
                    thread::sleep(wait_time);
                }
                last_hash = curr_hash;
            }
            thread::sleep(wait_time);
        }
        if let Ok(new_len) = stream.read(&mut recv_buf[recv_len..]) {
            if new_len > 0 {
                info!("收到数据长度: {}", new_len);
                match decode(&mut recv_buf[..(recv_len + new_len)]) {
                    Ok((len, Some(data))) => {
                        last_hash = calculate_hash(&data);
                        clip.set_contents(data).ok();
                        recv_len = len;
                    }
                    Ok((len, None)) => recv_len = len,
                    Err(e) => warn!("{}", e),
                }
                thread::sleep(wait_time);
            }
        }
        thread::sleep(wait_time);
    }
    Ok(())
}
