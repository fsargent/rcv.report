use colored::*;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::ffi::OsString;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Read;
use std::io::{BufReader, BufWriter};
use std::path::Path;

/// Read a JSON-serialized file into an object. Applies GZ decompression
/// if the file path ends in `.gz`.
pub fn read_serialized<T: DeserializeOwned>(path: &Path) -> T {
    eprintln!("Reading {}", path.to_str().unwrap().bright_blue());
    let file = File::open(path).unwrap();

    if path.extension() == Some(&OsString::from("gz")) {
        // For some reason, reading from a BufReader fails so we instead
        // load the whole file into memory. As it happens, this should be
        // faster anyway because of
        // https://github.com/serde-rs/json/issues/160
        let mut gzfile = GzDecoder::new(file);
        let mut contents = String::new();
        gzfile.read_to_string(&mut contents).unwrap();
        serde_json::from_str(&contents).unwrap()
    } else {
        let reader = BufReader::new(file);
        serde_json::from_reader(reader).unwrap()
    }
}

/// Write the given object as JSON. Applies GZ compression if the file
/// path ends in `.gz`. Creates the file if it doesn't exist, otherwise
/// overwrites it.
pub fn write_serialized<T: Serialize>(path: &Path, value: &T) {
    eprintln!("Writing {}", path.to_str().unwrap().bright_blue());

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .unwrap();

    if path.extension() == Some(&OsString::from("gz")) {
        let gzfile = GzEncoder::new(file, Compression::best());
        let writer = BufWriter::new(gzfile);
        serde_json::to_writer(writer, &value).unwrap();
    } else {
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &value).unwrap();
    }
}
