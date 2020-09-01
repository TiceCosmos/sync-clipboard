use copypasta::{ClipboardContext, ClipboardProvider};
use log::info;
use std::array::TryFromSliceError;
use std::collections::hash_map::DefaultHasher;
use std::convert::TryInto;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::io::prelude::*;
use std::net::TcpStream;

pub const WAIT_MS: u64 = 1000;
pub const BUF_LEN: usize = 10240;

pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

pub fn encode(buf: &mut [u8], s: String) -> Option<usize> {
    let data = s.into_bytes();
    if data.is_empty() {
        return None;
    }
    let len = (data.len() as u16).to_be_bytes();
    buf[0] = 0xff;
    buf[1] = 0xff;
    buf[2] = len[0];
    buf[3] = len[1];
    let mut n = 4;
    for (a, b) in buf[4..].iter_mut().zip(data) {
        *a = b;
        n += 1;
    }
    Some(n)
}

pub fn decode(buf: &mut [u8]) -> Result<(usize, Option<String>), TryFromSliceError> {
    let len = buf.len();
    if len < 4 {
        return Ok((len, None));
    }
    let mut s = 0;
    let mut proxy = u16::from_be_bytes(buf[s..(s + 2)].try_into()?);
    while proxy != 0xffff && s + 4 < len {
        s += 1;
        proxy = u16::from_be_bytes(buf[s..(s + 2)].try_into()?);
    }
    let mut cont = None;
    if proxy == 0xffff && s + 4 < len {
        let count = u16::from_be_bytes(buf[(s + 2)..(s + 4)].try_into()?) as usize;
        if s + 4 + count <= len {
            cont = Some(
                String::from_utf8_lossy(buf[(s + 4)..(s + 4 + count)].try_into()?).into_owned(),
            );
            s += 4 + count;
        }
    };
    if s != 0 {
        for (i, j) in (s..len).enumerate() {
            buf[i] = buf[j];
        }
    }
    Ok((len - s, cont))
}

pub fn stream_recv<F>(
    stream: &mut TcpStream,
    buffer: &mut [u8],
    start: usize,
    mut func: F,
) -> Result<usize, Box<dyn Error>>
where
    F: FnMut(String) -> Result<(), Box<dyn Error>>,
{
    if let Ok(new_len) = stream.read(&mut buffer[start..]) {
        if new_len > 0 {
            info!("收到数据长度: {}", new_len);
            match decode(&mut buffer[..(start + new_len)])? {
                (len, Some(data)) => {
                    func(data)?;
                    return Ok(len);
                }
                (len, None) => return Ok(len),
            }
        }
    }
    Ok(start)
}

pub fn clipboard_check<F>(
    clipboard: &mut ClipboardContext,
    last_hash: &mut u64,
    mut func: F,
) -> Result<(), Box<dyn Error>>
where
    F: FnMut(String) -> Result<(), Box<dyn Error>>,
{
    let data = clipboard.get_contents()?;
    let curr_hash = calculate_hash(&data);
    if data.starts_with("x-special/") {
        *last_hash = curr_hash;
    }
    if last_hash != &curr_hash {
        func(data)?;
        *last_hash = curr_hash;
    }
    Ok(())
}
