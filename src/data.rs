use std::fs;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use racing_scraper::get_racing_data;
use racing_scraper::models::{EventDetail, EventType};
use tracing::trace;

use crate::linear::Matrix;

const PODIUM: usize = 4;

pub trait EventDetailExt {
    fn summarise(self) -> RaceSummary;
}
impl EventDetailExt for EventDetail {
    fn summarise(self) -> RaceSummary {
        let mut prices = Matrix::allocate(PODIUM, self.runners.len());
        for rank in 0..PODIUM {
            let row_slice = prices.row_slice_mut(rank);
            for (runner_index, runner_data) in self.runners.iter().enumerate() {
                row_slice[runner_index] = match runner_data.prices.as_ref() {
                    None => f64::INFINITY,
                    Some(prices) => {
                        let price = match rank {
                            0 => prices.win,
                            1 => prices.top2,
                            2 => prices.top3,
                            3 => prices.top4,
                            _ => unimplemented!()
                        };
                        price as f64
                    }
                }
            }
        }
        RaceSummary {
            id: self.id,
            race_name: self.race_name,
            meeting_name: self.meeting_name,
            race_type: self.race_type,
            race_number: self.race_number,
            capture_time: self.capture_time,
            places_paying: self.places_paying as usize,
            class_name: self.class_name,
            prices,
        }
    }
}

#[derive(Debug)]
pub struct RaceSummary {
    pub id: u64,
    pub race_name: String,
    pub meeting_name: String,
    pub race_type: EventType,
    pub race_number: u8,
    pub capture_time: DateTime<Utc>,
    pub places_paying: usize,
    pub class_name: String,
    pub prices: Matrix<f64>,
}

pub fn read_from_file(path: impl AsRef<Path>) -> anyhow::Result<EventDetail> {
    let file = File::open(path)?;
    trace!("reading from {file:?}");
    let event_detail = serde_json::from_reader(file)?;
    Ok(event_detail)
}

#[derive(Debug)]
pub enum Predicate {
    Type { race_type: EventType }
}
impl Predicate {
    pub fn closure(self) -> impl FnMut(&EventDetail) -> bool {
        move |event| {
            match &self {
                Predicate::Type { race_type: event_type } => {
                    &event.race_type == event_type
                }
            }
        }
    }
}

pub type PredicateClosure = Box<dyn FnMut(&EventDetail) -> bool>;

pub struct PredicateClosures {
    closures: Vec<PredicateClosure>
}

impl <P: Into<PredicateClosure>> From<Vec<P>> for PredicateClosures {
    fn from(closurelikes: Vec<P>) -> Self {
        Self {
            closures: closurelikes.into_iter().map(Into::into).collect()
        }
    }
}

impl From<PredicateClosures> for PredicateClosure {
    fn from(mut predicates: PredicateClosures) -> Self {
        Box::new(move |event_detail| {
            for closure in predicates.closures.iter_mut() {
                if !closure(event_detail) {
                    return false;
                }
            }
            true
        })
    }
}

impl From<Predicate> for PredicateClosure {
    fn from(predicate: Predicate) -> Self {
        Box::new(predicate.closure())
    }
}

pub fn read_from_dir(path: impl AsRef<Path>, closurelike: impl Into<PredicateClosure>) -> anyhow::Result<Vec<EventDetail>> {
    let mut files = vec![];
    recurse_dir(path.as_ref().into(), &mut files)?;
    let mut races = Vec::with_capacity(files.len());
    let mut closure = closurelike.into();
    for file in files {
        let race = read_from_file(file)?;
        if closure(&race) {
            races.push(race);
        }
    }
    Ok(races)
}

fn recurse_dir(path: PathBuf, files: &mut Vec<PathBuf>) -> anyhow::Result<()>  {
    let md = fs::metadata(&path)?;
    if md.is_dir() {
        let entries = fs::read_dir(path)?;
        for entry in entries {
            recurse_dir(entry?.path(), files)?;
        }
    } else if path.extension().unwrap_or_default() == "json" {
        files.push(path);
    }
    Ok(())
}

pub async fn download_by_id(id: u64) -> anyhow::Result<EventDetail> {
    let event_detail = get_racing_data(&id).await?;
    Ok(event_detail)
}

pub struct CsvFile {
    writer: BufWriter<File>
}
impl CsvFile {
    pub fn create(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        Ok(Self { writer })
    }

    pub fn append<'a, S>(&mut self, record: Vec<S>) -> anyhow::Result<()> where S: AsRef<str> {
        for (index, datum) in record.iter().enumerate() {
            let str: &str = datum.as_ref();
            self.writer.write_all(str.as_bytes())?;
            if index == record.len() - 1 {
                self.writer.write_all("\n".as_bytes())?;
            } else {
                self.writer.write_all(",".as_bytes())?;
            }
        }
        Ok(())
    }

    pub fn flush(&mut self) -> anyhow::Result<()> {
        Ok(self.writer.flush()?)
    }
}