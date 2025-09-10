use sha1::{Digest, Sha1};
use std::fs::File;
use std::io;
use std::path::PathBuf;

/// Return the SHA-1 hash of the file at the given location.
pub fn hash_file(path: PathBuf) -> String {
    let mut file = File::open(path).unwrap();
    let mut hasher = Sha1::new();
    io::copy(&mut file, &mut hasher).unwrap();
    let hash = hasher.finalize();
    format!("{:x}", hash)
}
