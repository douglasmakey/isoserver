use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};

pub fn random_suffix(len: usize) -> String {
    Alphanumeric.sample_string(&mut thread_rng(), len)
}
