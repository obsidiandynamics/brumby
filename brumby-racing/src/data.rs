use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use racing_scraper::get_racing_data;
use racing_scraper::models::{EventDetail, EventType};

use brumby::file;
use brumby::file::ReadJsonFile;
use brumby::linear::matrix::Matrix;

const PODIUM: usize = 4;

#[derive(Debug)]
pub struct PlacePriceDeparture {
    pub root_mean_sq: f64,
    pub worst: f64,
}

pub trait EventDetailExt {
    fn place_price_departure(&self) -> PlacePriceDeparture;
}
impl EventDetailExt for EventDetail {
    fn place_price_departure(&self) -> PlacePriceDeparture {
        let mut sum_sq = 0.;
        let mut worst_sq = 0.;
        let mut active_runners = 0;

        fn relative_delta(a: f64, b: f64) -> f64 {
            (a - b) / f64::max(a, b)
        }
        for runner in &self.runners {
            if let Some(prices) = &runner.prices {
                active_runners += 1;
                let corresponding_top_price = match self.places_paying {
                    2 => prices.top2,
                    3 => prices.top3,
                    4 => prices.top4,
                    other => unimplemented!("unsupported number of places paying {other}"),
                };
                let departure_sq =
                    relative_delta(corresponding_top_price as f64, prices.place as f64).powi(2);
                sum_sq += departure_sq;
                if departure_sq > worst_sq {
                    worst_sq = departure_sq;
                }
            }
        }
        assert!(active_runners > 0, "no active runners");

        let root_mean_sq = (sum_sq / active_runners as f64).sqrt();
        let worst = worst_sq.sqrt();
        PlacePriceDeparture {
            root_mean_sq,
            worst,
        }
    }
}

impl From<EventDetail> for RaceSummary {
    fn from(external: EventDetail) -> Self {
        let mut prices = Matrix::allocate(PODIUM, external.runners.len());
        for rank in 0..PODIUM {
            let row_slice = prices.row_slice_mut(rank);
            for (runner_index, runner_data) in external.runners.iter().enumerate() {
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
        Self {
            id: external.id,
            race_name: external.race_name,
            meeting_name: external.meeting_name,
            race_type: external.race_type,
            race_number: external.race_number,
            capture_time: external.capture_time,
            places_paying: external.places_paying as usize,
            class_name: external.class_name,
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

#[derive(Debug)]
pub enum Predicate {
    Type { race_type: EventType },
    Departure { cutoff_worst: f64 },
}
impl Predicate {
    pub fn closure(self) -> impl FnMut(&EventDetail) -> bool {
        move |event| match &self {
            Predicate::Type {
                race_type: event_type,
            } => &event.race_type == event_type,
            Predicate::Departure { cutoff_worst } => {
                event.place_price_departure().worst <= *cutoff_worst
            }
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
