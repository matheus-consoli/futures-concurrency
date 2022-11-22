//! Utilities to implement the different futures of this crate.

mod array;
mod pin;
mod poll_state;
mod rng;
mod tuple;
mod wakers;
mod indexer;

pub(crate) use array::array_assume_init;
pub(crate) use pin::{get_pin_mut, get_pin_mut_from_vec, iter_pin_mut, iter_pin_mut_vec};
pub(crate) use poll_state::MaybeDone;
pub(crate) use poll_state::{PollArray, PollState, PollVec};
pub(crate) use rng::RandomGenerator;
pub(crate) use tuple::{gen_conditions, permutations};
pub(crate) use wakers::{WakerArray, WakerVec};
pub(crate) use indexer::Indexer;


#[cfg(test)]
pub(crate) use wakers::DummyWaker;

#[cfg(test)]
pub(crate) mod channel;
