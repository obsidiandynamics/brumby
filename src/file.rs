//! File and directory manipulation utilities.

use std::fs::File;
use std::{fs, io};
use std::ffi::OsStr;
use std::io::Error;
use std::path::{Path, PathBuf};
use serde::de::DeserializeOwned;
use serde_json::from_reader;

/// Reads a JSON-encoded type from a given file `path`.
pub fn read_json<D: DeserializeOwned>(path: impl AsRef<Path>) -> Result<D, io::Error> {
    let file = File::open(path)?;
    Ok(from_reader(file)?)
}

pub trait FromJsonFile<D> {
    fn from_json_file(path: impl AsRef<Path>) -> Result<D, io::Error>;
}

impl<D: DeserializeOwned> FromJsonFile<D> for D {
    fn from_json_file(path: impl AsRef<Path>) -> Result<D, Error> {
        read_json(path)
    }
}

/// Recursively locates all files in a given directory matching the supplied `extension_filter`. The
/// located files are written into the `files` vector. If the given `path` is a file that matches the
/// filter (rather than a directory), it is added to `files`.
pub fn recurse_dir(path: PathBuf, files: &mut Vec<PathBuf>, extension_filter: &mut impl FnMut(&OsStr) -> bool) -> Result<(), io::Error> {
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