mod attractors;
pub mod render;
mod sampler;
pub mod ui;

pub use crate::attractors::*;
pub use crate::sampler::*;

/// Calls `$m!` with every attractor type, so the program, tests and examples share one list.
/// Add new attractors here.
#[macro_export]
macro_rules! all_attractors {
    ($m:ident) => {
        $m!(
            $crate::CliffordAttractor,
            $crate::DeJongAttractor,
            $crate::KingsDreamAttractor,
            $crate::HenonAttractor,
            $crate::TinkerbellAttractor,
            $crate::SprottQuadraticAttractor,
            $crate::HopalongAttractor,
            $crate::GumowskiMiraAttractor,
            $crate::IkedaAttractor,
            $crate::DuffingAttractor,
            $crate::LoziAttractor,
            $crate::BedheadAttractor,
            $crate::SvenssonAttractor
        )
    };
}
