//! guardian: forward-collision warning arbiter.
//!
//! Skeleton first: types, signatures, doc contracts, `todo!()` bodies.
//! The one shared type is below. The runner's metrics and the
//! comparisons rely on its ordering.

/// The arbiter's per-cycle output.
///
/// Ordering is severity: `NoAction < Warn < Brake`. Keep the derive;
/// comparisons between decisions rely on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Decision {
    NoAction,
    Warn,
    Brake,
}

/// Stable adapter used by the scenario runner and independent evaluator.
///
/// Your implementation chooses the concrete state, thresholds, and failure
/// policy. Construct it through `Default`; the evaluator calls one cycle at a
/// time through this trait.
pub trait Arbiter: Default {
    /// Produce exactly one decision for the current 50 ms cycle.
    fn decide(&mut self, reports: &[sensor_sim::TrackReport]) -> Decision;
}

// Your skeleton starts here.
