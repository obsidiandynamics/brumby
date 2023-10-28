//! Utilities for working with CSV files.

use std::borrow::Cow;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader, BufWriter, Lines, Write};
use std::ops::{Index, IndexMut};
use std::path::Path;

pub struct CsvWriter {
    writer: BufWriter<File>,
}
impl CsvWriter {
    pub fn create(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        Ok(Self { writer })
    }

    pub fn append<'a, R>(&mut self, record: R) -> Result<(), io::Error>
    where
        R: IntoIterator,
        R::Item: AsRef<str>,
    {
        let mut first = true;
        for datum in record.into_iter() {
            if first {
                first = false;
            } else {
                self.writer.write_all(",".as_bytes())?;
            }
            let str: &str = datum.as_ref();
            self.writer.write_all(str.as_bytes())?;
        }
        self.writer.write_all("\n".as_bytes())?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), io::Error> {
        self.writer.flush()
    }
}

pub struct CsvReader {
    lines: Lines<BufReader<File>>,
}
impl CsvReader {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        let file = File::open(path)?;
        let lines = BufReader::new(file).lines();
        Ok(Self { lines })
    }

    pub fn read(&mut self) -> Option<Result<Vec<String>, io::Error>> {
        self.lines
            .next()
            .map(|line| line.map(|line| line.split(',').map(ToString::to_string).collect()))
    }
}

impl Iterator for CsvReader {
    type Item = Result<Vec<String>, io::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.read()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    items: Vec<Cow<'static, str>>,
}
impl Record {
    pub fn with_capacity(capacity: usize) -> Self {
        let mut items = Vec::with_capacity(capacity);
        items.resize_with(capacity, || Cow::Borrowed(""));
        Self { items }
    }

    pub fn with_values<I>(values: I) -> Self
    where
        I: IntoIterator,
        I::Item: ToString,
    {
        let items = values
            .into_iter()
            .map(|value| Cow::Owned(value.to_string()))
            .collect();
        Self { items }
    }

    pub fn set(&mut self, ordinal: impl Into<usize>, value: impl ToString) {
        self.items[ordinal.into()] = Cow::Owned(value.to_string())
    }
}

impl IntoIterator for Record {
    type Item = Cow<'static, str>;
    type IntoIter = alloc::vec::IntoIter<Cow<'static, str>>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<I: Into<usize>> Index<I> for Record {
    type Output = Cow<'static, str>;

    fn index(&self, index: I) -> &Self::Output {
        &self.items[index.into()]
    }
}

impl<I: Into<usize>> IndexMut<I> for Record {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.items[index.into()]
    }
}
