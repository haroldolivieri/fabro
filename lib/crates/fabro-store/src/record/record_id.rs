use std::fmt::Write;

use fabro_types::{RunBlobId, RunId};

use super::RecordId;
use crate::{Error, Result};

impl RecordId for [u8; 32] {
    fn key_segments(&self) -> Vec<String> {
        let mut encoded = String::with_capacity(self.len() * 2);
        for byte in self {
            write!(&mut encoded, "{byte:02x}").expect("write to String cannot fail");
        }
        vec![encoded]
    }

    fn from_key_segments(segs: &[&str]) -> Result<Self> {
        let [segment] = segs else {
            return Err(Error::KeyParse(format!(
                "expected 1 segment for [u8; 32], got {}",
                segs.len()
            )));
        };

        if segment.len() != 64 {
            return Err(Error::KeyParse(format!(
                "expected 64 hex characters for [u8; 32], got {}",
                segment.len()
            )));
        }

        let mut bytes = [0_u8; 32];
        for (index, chunk) in segment.as_bytes().chunks_exact(2).enumerate() {
            let chunk = std::str::from_utf8(chunk).map_err(|err| {
                Error::KeyParse(format!("hex segment was not valid UTF-8: {err}"))
            })?;
            bytes[index] = u8::from_str_radix(chunk, 16)
                .map_err(|err| Error::KeyParse(format!("invalid hex byte {chunk:?}: {err}")))?;
        }
        Ok(bytes)
    }
}

impl RecordId for String {
    fn key_segments(&self) -> Vec<String> {
        vec![self.clone()]
    }

    fn from_key_segments(segs: &[&str]) -> Result<Self> {
        let [segment] = segs else {
            return Err(Error::KeyParse(format!(
                "expected 1 segment for String, got {}",
                segs.len()
            )));
        };
        Ok((*segment).to_string())
    }
}

impl RecordId for RunBlobId {
    fn key_segments(&self) -> Vec<String> {
        vec![self.to_string()]
    }

    fn from_key_segments(segs: &[&str]) -> Result<Self> {
        let [segment] = segs else {
            return Err(Error::KeyParse(format!(
                "expected 1 segment for RunBlobId, got {}",
                segs.len()
            )));
        };
        segment
            .parse()
            .map_err(|err| Error::KeyParse(format!("invalid RunBlobId segment {segment:?}: {err}")))
    }
}

impl RecordId for RunId {
    fn key_segments(&self) -> Vec<String> {
        vec![
            self.created_at().format("%Y-%m-%d").to_string(),
            self.to_string(),
        ]
    }

    fn from_key_segments(segs: &[&str]) -> Result<Self> {
        if segs.len() != 2 {
            return Err(Error::KeyParse(format!(
                "expected 2 segments for RunId, got {}",
                segs.len()
            )));
        }
        segs[1]
            .parse()
            .map_err(|err| Error::KeyParse(format!("invalid RunId segment {:?}: {err}", segs[1])))
    }
}
