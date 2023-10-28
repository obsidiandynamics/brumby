//! A fast, allocation-free Monte Carlo model of a top-_N_ podium finish in racing events.
//! Derives probabilities for placing in arbitrary positions given only win probabilities.
//! Also derives joint probability of multiple runners with arbitrary (exact and top-_N_) placings.

#![allow(clippy::too_many_arguments)]

pub mod capture;
pub mod data;
pub mod display;
pub mod fit;
pub mod linear;
pub mod market;
pub mod mc;
pub mod opt;
pub mod print;
pub mod probs;
pub mod selection;

#[cfg(test)]
pub(crate) mod testing;

#[doc = include_str!("../README.md")]
#[cfg(doc)]
fn readme() {}
