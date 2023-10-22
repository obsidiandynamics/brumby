use std::fs::File;
use std::path::Path;
use chrono::{DateTime, Utc};
use racing_scraper::models::{EventDetail, EventType};
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
                    None => 0.0,
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
            places_paying: self.places_paying,
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
    pub places_paying: u8,
    pub class_name: String,
    pub prices: Matrix<f64>,
}

pub fn read_from_file(path: impl AsRef<Path>) -> anyhow::Result<EventDetail> {
    let file = File::open(path)?;
    let event_detail = serde_json::from_reader(file)?;
    Ok(event_detail)
}