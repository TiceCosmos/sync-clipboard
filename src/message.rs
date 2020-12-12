use crate::Result;
use std::{array::TryFromSliceError, convert::TryInto};

pub fn encode(buf: &mut [u8], s: String) -> Option<usize> {
    let data = s.into_bytes();
    if data.is_empty() {
        return None;
    }
    let len = (data.len() as u32).to_be_bytes();
    for (t, l) in buf.iter_mut().zip(len.iter()) {
        *t = *l;
    }
    let mut n = 4;
    for (a, b) in buf[4..].iter_mut().zip(data) {
        *a = b;
        n += 1;
    }
    Some(n)
}

pub fn decode(buf: &mut [u8]) -> Result<Option<(usize, String)>, TryFromSliceError> {
    let len = buf.len();
    if len < 4 {
        return Ok(None);
    }

    let n = u32::from_be_bytes(buf[..4].try_into()?) as usize;
    let mut data = None;
    if n + 4 <= len {
        data = Some((
            n + 4,
            String::from_utf8_lossy(buf[4..(n + 4)].try_into()?).into_owned(),
        ));
        buf.copy_within((n + 4).., 0);
    }

    Ok(data)
}
