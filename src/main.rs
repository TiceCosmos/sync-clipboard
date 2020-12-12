mod client;
mod clipboard;
mod message;
mod server;
mod utils;

use crate::{client::Client, clipboard::Clipboard, server::Server};

use async_std::{
    channel::{self, RecvError, SendError},
    io,
    net::TcpStream,
    prelude::*,
    sync::{Arc, Mutex},
};
use chrono::Local;
use env_logger::Builder;
use futures_lite::io::WriteHalf;
use log::info;
use std::{
    collections::HashMap,
    io::prelude::*,
    net::{IpAddr, SocketAddr},
};
use structopt::StructOpt;

type Result<T, E = Error> = std::result::Result<T, E>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, WriteHalf<TcpStream>>>>;

const WAIT_MS: u64 = 1000;
const RECON_S: u64 = 2;
const BUF_LEN: usize = 10240;

#[derive(StructOpt, Debug)]
struct Opt {
    /// auto open http url
    #[structopt(short = "o", long = "open")]
    open: bool,
    /// client/server, defalut is client
    #[structopt(short = "s", long = "server")]
    serv: bool,
    /// server addr
    #[structopt(short = "a", long = "addr", default_value = "127.0.0.1")]
    addr: IpAddr,
    /// server port
    #[structopt(short = "p", long = "port", default_value = "43190")]
    port: u16,
}

#[async_std::main]
async fn main() -> Result<()> {
    Builder::from_default_env()
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                Local::now().format("%FT%T"),
                buf.default_styled_level(record.level()),
                record.args()
            )
        })
        .init();

    let opt = Opt::from_args();
    info!("{:#?}", opt);

    let addr = SocketAddr::new(opt.addr, opt.port);

    let (sx, rx) = channel::bounded(4);

    let peer_map = PeerMap::default();

    let mut clipboard = Clipboard::new(rx, peer_map.clone())?;

    if opt.serv {
        let mut server = Server::new(addr, sx, peer_map.clone()).await?;
        if opt.open {
            clipboard.run().race(server.with_auto().run()).await;
        } else {
            clipboard.run().race(server.run()).await;
        }
    } else {
        let mut client = Client::new(addr, sx, peer_map.clone());
        if opt.open {
            clipboard.run().race(client.with_auto().run()).await;
        } else {
            clipboard.run().race(client.run()).await;
        }
    }

    Ok(())
}

#[derive(Debug)]
pub enum Error {
    Lnk(io::Error),
    Snd(SendError<String>),
    Rcv(RecvError),
    Dyn(Box<dyn std::error::Error>),
}
impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Lnk(e)
    }
}
impl From<SendError<String>> for Error {
    fn from(e: SendError<String>) -> Self {
        Self::Snd(e)
    }
}
impl From<RecvError> for Error {
    fn from(e: RecvError) -> Self {
        Self::Rcv(e)
    }
}
impl From<Box<dyn std::error::Error>> for Error {
    fn from(e: Box<dyn std::error::Error>) -> Self {
        Self::Dyn(e)
    }
}
