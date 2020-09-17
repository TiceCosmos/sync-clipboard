pub mod client;
pub mod common;
pub mod server;

use client::*;
use server::*;

use chrono::Local;
use env_logger::Builder;
use log::info;
use std::{error::Error, io::prelude::*};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opt {
    /// auto open http url
    #[structopt(short = "o", long = "open")]
    open: bool,
    /// server addr
    #[structopt(short = "s", long = "server")]
    serv: bool,
    /// server addr
    #[structopt(short = "a", long = "addr", default_value = "127.0.0.1")]
    addr: String,
    /// server port
    #[structopt(short = "p", long = "port", default_value = "43190")]
    port: u16,
}

fn main() -> Result<(), Box<dyn Error>> {
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

    if opt.serv {
        Server::bind(opt.addr, opt.port)?.cycle(opt.open);
    } else {
        Client::bind(opt.addr, opt.port)?.cycle(opt.open);
    }

    Ok(())
}
