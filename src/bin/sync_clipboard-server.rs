use copypasta::{ClipboardContext, ClipboardProvider};
use log::{info, warn};
use std::error::Error;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::{thread, time};
use structopt::StructOpt;
use sync_clipboard::*;

static LINK_COUNT: AtomicUsize = AtomicUsize::new(0);

#[derive(StructOpt, Debug)]
struct Opt {
    /// server port
    #[structopt(short = "p", long = "port", default_value = "43190")]
    port: u16,
}

fn main() -> std::io::Result<()> {
    env_logger_init();

    let opt = Opt::from_args();
    info!("{:#?}", opt);

    let listener = TcpListener::bind(("0.0.0.0", opt.port))?;

    let (send_l, recv_l) = channel();
    let (send_r, recv_r) = channel();

    thread::spawn(move || monitor_clipboard(send_l, recv_r));

    let recv_l = Arc::new(Mutex::new(recv_l));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream, &recv_l, &send_r),
            Err(e) => warn!("{}", e),
        }
    }

    Ok(())
}

fn monitor_clipboard(
    send_l: Sender<String>,   // local clipboard context
    recv_r: Receiver<String>, // remote clipboard context
) {
    if let Ok(mut clip) = ClipboardContext::new() {
        let wait_time = time::Duration::from_millis(WAIT_MS);

        let mut last_hash = calculate_hash(&String::new());

        'outer: while let Ok(data) = clip.get_contents() {
            let curr_hash = calculate_hash(&data);
            if last_hash != curr_hash {
                let len = LINK_COUNT.load(Ordering::Relaxed);
                for _ in 0..len {
                    if let Err(e) = send_l.send(data.clone()) {
                        warn!("{}", e);
                        break 'outer;
                    }
                }
                last_hash = curr_hash;
            }
            if let Ok(data) = recv_r.recv_timeout(wait_time) {
                last_hash = calculate_hash(&data);
                clip.set_contents(data).ok();
            }
            thread::sleep(wait_time * 2);
        }
    }
    drop(send_l);
}

fn handle_client(
    stream: TcpStream,
    recv_l: &Arc<Mutex<Receiver<String>>>, // local clipboard context
    send_r: &Sender<String>,               // remote clipboard context
) {
    let recv_l = recv_l.clone();
    let send_r = send_r.clone();
    thread::spawn(move || {
        if let Ok(source) = stream.local_addr() {
            info!("{:?} linked", source);
            LINK_COUNT.fetch_add(1, Ordering::Relaxed);
            if let Err(e) = synchronize(stream, recv_l, send_r) {
                warn!("{}", e);
            }
            LINK_COUNT.fetch_sub(1, Ordering::Relaxed);
            info!("{:?} unlink", source);
        }
    });
}

fn synchronize(
    mut stream: TcpStream,
    recv_l: Arc<Mutex<Receiver<String>>>, // local clipboard context
    send_r: Sender<String>,               // remote clipboard context
) -> Result<(), Box<dyn Error>> {
    let wait_time = time::Duration::from_millis(WAIT_MS);
    stream.set_read_timeout(Some(wait_time))?;

    let mut send_buf = [0xff; 1024];
    let mut recv_buf = [0; 2048];
    let mut recv_len = 0;
    loop {
        if let Ok(data) = if let Ok(receiver) = recv_l.lock() {
            receiver.recv_timeout(wait_time)
        } else {
            break;
        } {
            if let Some(n) = encode(&mut send_buf, data) {
                if stream.write(&send_buf[0..n]).is_err() {
                    break;
                }
                thread::sleep(wait_time);
            }
        }
        if let Ok(new_len) = stream.read(&mut recv_buf[recv_len..]) {
            if new_len > 0 {
                info!("收到数据长度: {}", new_len);
                match decode(&mut recv_buf[..(recv_len + new_len)]) {
                    Ok((len, Some(data))) => {
                        if send_r.send(data).is_err() {
                            break;
                        }
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
