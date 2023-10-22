//! A fast, allocation-free Monte Carlo model of a top-_N_ podium finish in racing events.
//! Derives probabilities for placing in arbitrary positions given only win probabilities.
//! Also derives joint probability of multiple runners with arbitrary (exact and top-_N_) placings.

pub mod capture;
pub mod linear;
pub mod mc;
pub mod opt;
pub mod probs;
pub mod selection;

#[doc = include_str!("../README.md")]
#[cfg(doc)]
fn readme() {}
