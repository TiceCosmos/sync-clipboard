use chrono::Local;
use env_logger::Builder;
use std::array::TryFromSliceError;
use std::collections::hash_map::DefaultHasher;
use std::convert::TryInto;
use std::hash::{Hash, Hasher};
use std::io::Write;

pub const WAIT_MS: u64 = 200;

pub fn env_logger_init() {
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

pub fn calculate_hash<T: Hash>(t: &T) -> u64 {
    let mut s = DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}
