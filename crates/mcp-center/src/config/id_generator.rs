use std::collections::HashSet;

use rand::{
    distributions::{Alphanumeric, DistString},
    thread_rng,
};

/// Generate a short, unique id using the provided existing IDs as guard.
pub fn generate_id(existing: &HashSet<String>) -> String {
    let mut rng = thread_rng();

    loop {
        let candidate = Alphanumeric.sample_string(&mut rng, 8).to_lowercase();
        if !existing.contains(&candidate) {
            return candidate;
        }
    }
}
