//! File and directory manipulation utilities.

use std::fs;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Error;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{from_reader, to_writer_pretty};

/// Reads a JSON-encoded type from a given file `path`.
pub fn read_json<D: DeserializeOwned>(path: impl AsRef<Path>) -> Result<D, Error> {
    let file = File::open(path)?;
    Ok(from_reader(file)?)
}

// JSON-encodes the `value` in pretty-printed form and writes it to a given `path`.
pub fn write_json(path: impl AsRef<Path>, value: &impl Serialize) -> Result<(), Error> {
    let file = File::create(path)?;
    Ok(to_writer_pretty(file, value)?)
}

pub trait ReadJsonFile<D> {
    fn read_json_file(path: impl AsRef<Path>) -> Result<D, Error>;
}

impl<D: DeserializeOwned> ReadJsonFile<D> for D {
    fn read_json_file(path: impl AsRef<Path>) -> Result<D, Error> {
        read_json(path)
    }
}

pub trait WriteJsonFile<S: Serialize> {
    fn write_json_file(&self, path: impl AsRef<Path>) -> Result<(), Error>;
}

impl<S: Serialize> WriteJsonFile<S> for S {
    fn write_json_file(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        write_json(path, self)
    }
}

/// Recursively locates all files in a given directory matching the supplied `extension_filter`. The
/// located files are written into the `files` vector. If the given `path` is a file that matches the
/// filter (rather than a directory), it is added to `files`.
pub fn recurse_dir(path: PathBuf, files: &mut Vec<PathBuf>, extension_filter: &mut impl FnMut(&OsStr) -> bool) -> Result<(), Error> {
    let md = fs::metadata(&path)?;
    if md.is_dir() {
        let entries = fs::read_dir(path)?;
        for entry in entries {
            recurse_dir(entry?.path(), files, extension_filter)?;
        }
    } else if extension_filter(path.extension().unwrap_or_default()) {
        files.push(path);
    }
    Ok(())
}