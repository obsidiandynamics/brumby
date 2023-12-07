//! A fast, allocation-free Monte Carlo model of a top-_N_ podium finish in racing events.
//! Derives probabilities for placing in arbitrary positions given only win probabilities.
//! Also derives joint probability of multiple runners with arbitrary (exact and top-_N_) placings.

#![allow(clippy::too_many_arguments)]

pub mod capture;
pub mod comb;
pub mod csv;
pub mod display;
pub mod domain;
pub mod factorial;
pub mod file;
pub mod harville;
pub mod interval;
pub mod linear;
pub mod market;
pub mod mc;
pub mod model;
pub mod multinomial;
pub mod opt;
pub mod poisson;
pub mod print;
pub mod probs;
pub mod racing_data;
pub mod scoregrid;
pub mod selection;
pub mod soccer_data;
pub mod timed;

#[cfg(test)]
pub(crate) mod testing;

#[doc = include_str!("../README.md")]
#[cfg(doc)]
fn readme() {}
