//! A fast, allocation-free Monte Carlo model of a top-_N_ podium finish in racing events.
//! Derives probabilities for placing in arbitrary positions given only win probabilities.
//! Also derives joint probability of multiple runners with arbitrary (exact and top-_N_) placings.

#![allow(clippy::too_many_arguments)]

extern crate core;

pub mod arrays;
pub mod capture;
pub mod comb;
pub mod csv;
pub mod derived_price;
pub mod dilative;
pub mod display;
pub mod factorial;
pub mod feed_id;
pub mod file;
pub mod harville;
pub mod hash_lookup;
pub mod linear;
pub mod market;
pub mod mc;
pub mod multinomial;
pub mod opt;
pub mod poisson;
pub mod probs;
pub mod tables;
pub mod timed;
pub mod selection;
pub mod stack_vec;

#[doc = include_str!("../../README.md")]
#[cfg(doc)]
fn readme() {}
