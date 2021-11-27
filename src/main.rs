// Used to write attractors to files
mod attractors;
use crate::attractors::*;

/// Not really used just yet, mainly contains some primitive explorations
fn main() {
    let params = vec![1.5; 4];
    let mut clifford: CliffordAttractor = CliffordAttractor::new(params);
    let mut x = 0.0;
    let mut y = 0.0;
    clifford.step(&mut x, &mut y, 10);
    clifford.to_file("filename.txt".to_string());
}
