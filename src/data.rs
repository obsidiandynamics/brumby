use anyhow::bail;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use racing_scraper::get_racing_data;
use racing_scraper::models::{EventDetail, EventType};

use crate::file;
use crate::file::ReadJsonFile;
use crate::linear::matrix::Matrix;

const PODIUM: usize = 4;

pub trait EventDetailExt {
    fn summarise(self) -> RaceSummary;
    fn validate_place_price_equivalence(&self) -> Result<(), anyhow::Error>;
}
impl EventDetailExt for EventDetail {
    fn summarise(self) -> RaceSummary {
        let mut prices = Matrix::allocate(PODIUM, self.runners.len());
        for rank in 0..PODIUM {
            let row_slice = prices.row_slice_mut(rank);
            for (runner_index, runner_data) in self.runners.iter().enumerate() {
                row_slice[runner_index] = match &runner_data.prices {
                    None => f64::INFINITY,
                    Some(prices) => {
                        let price = match rank {
                            0 => prices.win,
                            1 => prices.top2,
                            2 => prices.top3,
                            3 => prices.top4,
                            _ => unimplemented!(),
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

    fn validate_place_price_equivalence(&self) -> Result<(), anyhow::Error> {
        for runner in &self.runners {
            if let Some(prices) = &runner.prices {
                let corresponding_top_price = match self.places_paying {
                    1 => prices.top2,
                    2 => prices.top3,
                    3 => prices.top4,
                    other => bail!("unsupported number of places paying {other}"),
                };
                if prices.place != corresponding_top_price {
                    bail!(
                        "place and top-{} prices do not match for runner r{}: {} vs {}",
                        self.places_paying,
                        runner.runner_number,
                        prices.place,
                        corresponding_top_price
                    );
                }
            }
        }
        Ok(())
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

#[derive(Debug)]
pub enum Predicate {
    Type { race_type: EventType },
}
impl Predicate {
    pub fn closure(self) -> impl FnMut(&EventDetail) -> bool {
        move |event| match &self {
            Predicate::Type {
                race_type: event_type,
            } => &event.race_type == event_type,
        }
    }
}

pub type PredicateClosure = Box<dyn FnMut(&EventDetail) -> bool>;

pub struct PredicateClosures {
    closures: Vec<PredicateClosure>,
}

impl<P: Into<PredicateClosure>> From<Vec<P>> for PredicateClosures {
    fn from(closurelikes: Vec<P>) -> Self {
        Self {
            closures: closurelikes.into_iter().map(Into::into).collect(),
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

#[derive(Debug)]
pub struct RaceFile {
    pub race: EventDetail,
    pub file: PathBuf,
}

pub fn read_from_dir(
    path: impl AsRef<Path>,
    closurelike: impl Into<PredicateClosure>,
) -> anyhow::Result<Vec<RaceFile>> {
    let mut files = vec![];
    file::recurse_dir(path.as_ref().into(), &mut files, &mut |ext| ext == "json")?;
    let mut races = Vec::with_capacity(files.len());
    let mut closure = closurelike.into();
    for file in files {
        let race = EventDetail::read_json_file(&file)?;
        if closure(&race) {
            races.push(RaceFile { race, file });
        }
    }
    Ok(races)
}

pub async fn download_by_id(id: u64) -> anyhow::Result<EventDetail> {
    let event_detail = get_racing_data(&id).await?;
    Ok(event_detail)
}
