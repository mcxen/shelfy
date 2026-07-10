use std::fs::File;
use std::io::Read;
use std::path::Path;

use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};

use crate::orden::filter::{set_var, Filter, FilterResult};
use crate::orden::resource::Resource;
use crate::orden::value::Value;

/// Calculates the hash of a file.
///
/// Mirrors `organize.filters.hash.Hash`. Supports md5 (default), sha1, sha256, sha512.
pub struct Hash {
    pub algorithm: String,
}

impl Hash {
    pub fn new(algorithm: String) -> Self {
        Self { algorithm }
    }
}

pub fn hash_file(path: &Path, algo: &str) -> Result<String, String> {
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    let mut buf = [0u8; 262144]; // 256KB buffer
    match algo.to_lowercase().as_str() {
        "md5" => {
            let mut h = Md5::new();
            loop {
                let n = f.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            Ok(hex::encode(h.finalize()))
        }
        "sha1" => {
            let mut h = Sha1::new();
            loop {
                let n = f.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            Ok(hex::encode(h.finalize()))
        }
        "sha256" => {
            let mut h = Sha256::new();
            loop {
                let n = f.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            Ok(hex::encode(h.finalize()))
        }
        "sha512" => {
            let mut h = Sha512::new();
            loop {
                let n = f.read(&mut buf).map_err(|e| e.to_string())?;
                if n == 0 {
                    break;
                }
                h.update(&buf[..n]);
            }
            Ok(hex::encode(h.finalize()))
        }
        other => Err(format!("Unknown hash algorithm: {}", other)),
    }
}

impl Filter for Hash {
    fn name(&self) -> &str {
        "hash"
    }
    fn supports_dirs(&self) -> bool {
        false
    }
    fn pipeline(&mut self, res: &mut Resource) -> Result<FilterResult, String> {
        let path = res.path.as_ref().ok_or("hash: no path")?;
        let h = hash_file(path, &self.algorithm)?;
        set_var(res, "hash", Value::Str(h));
        Ok(FilterResult::Match)
    }
}

/// Hash the first chunk (1024 bytes) of a file — used by duplicate detection.
pub fn hash_first_chunk(path: &Path, algo: &str) -> Result<String, String> {
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    let mut chunk = [0u8; 1024];
    let n = f.read(&mut chunk).map_err(|e| e.to_string())?;
    let bytes = &chunk[..n];
    match algo.to_lowercase().as_str() {
        "md5" => Ok(hex::encode(md5::Md5::digest(bytes))),
        "sha1" => Ok(hex::encode(sha1::Sha1::digest(bytes))),
        "sha256" => Ok(hex::encode(sha2::Sha256::digest(bytes))),
        other => Err(format!("Unknown hash algorithm: {}", other)),
    }
}
