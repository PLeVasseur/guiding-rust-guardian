//! Reviewable metric semantics for Project C.
//!
//! This module deliberately contains no arbiter policy. It evaluates any
//! `Arbiter + Default`, including the participant implementation and test
//! doubles. A measured rate describes this seed batch; it is not a formal
//! confidence statement about a population.

use guardian::{Arbiter, Decision};
use sensor_sim::{Scenario, Sim, SimConfig};

const HARD_SEED_BASE: u64 = 1_000_000;
const EMPTY_SEED_BASE: u64 = 2_000_000;
const CUT_IN_SEED_BASE: u64 = 3_000_000;
const FINAL_BRAKE_WINDOW: usize = 20;
const WARN_TTC_S: f64 = 2.5;
const BRAKE_TTC_S: f64 = 1.4;
const MIN_RESPONSE_COVERAGE_PCT: f64 = 95.0;

#[derive(Debug)]
pub(crate) struct MetricsReport {
    trials: u64,
    hard_brake_rate_pct: f64,
    empty_false_alarm_rate_pct: f64,
    warn_latency_median_cycles: Option<f64>,
    brake_latency_median_cycles: Option<f64>,
    warn_response_coverage_pct: f64,
    brake_response_coverage_pct: f64,
    hard_chatter_transitions_per_run: f64,
    cut_in_chatter_transitions_per_run: f64,
    failures: Vec<Failure>,
}

#[derive(Debug)]
struct Failure {
    scenario: &'static str,
    seed: u64,
    reason: &'static str,
}

fn config(seed: u64) -> SimConfig {
    SimConfig {
        seed: Some(seed),
        ..SimConfig::default()
    }
}

fn decisions<A: Arbiter + Default>(scenario: Scenario, seed: u64) -> Vec<Decision> {
    let mut arbiter = A::default();
    Sim::new(scenario, config(seed))
        .map(|reports| arbiter.decide(&reports))
        .collect()
}

fn truth_ttc(scenario: Scenario) -> Vec<f64> {
    Sim::new(scenario, SimConfig::noiseless())
        .map(|reports| {
            reports
                .iter()
                .filter_map(|report| {
                    let closing_mps = -report.range_rate_mps;
                    (closing_mps > 0.1).then_some(report.range_m / closing_mps)
                })
                .fold(f64::INFINITY, f64::min)
        })
        .collect()
}

fn first_below(values: &[f64], threshold: f64) -> Option<usize> {
    values.iter().position(|value| *value < threshold)
}

fn ends_in_brake(decisions: &[Decision]) -> bool {
    decisions[decisions.len().saturating_sub(FINAL_BRAKE_WINDOW)..].contains(&Decision::Brake)
}

fn has_false_alarm(decisions: &[Decision]) -> bool {
    decisions.iter().any(|decision| *decision >= Decision::Warn)
}

fn latency_at_or_after(
    decisions: &[Decision],
    crossing: usize,
    required: Decision,
) -> Option<usize> {
    decisions
        .iter()
        .enumerate()
        .skip(crossing)
        .find(|(_, decision)| **decision >= required)
        .map(|(cycle, _)| cycle - crossing)
}

fn transitions(decisions: &[Decision]) -> usize {
    decisions
        .windows(2)
        .filter(|pair| pair[0] != pair[1])
        .count()
}

fn median(values: &mut [usize]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let middle = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[middle - 1] + values[middle]) as f64 / 2.0)
    } else {
        Some(values[middle] as f64)
    }
}

fn pct(count: usize, total: u64) -> f64 {
    100.0 * count as f64 / total as f64
}

pub(crate) fn evaluate<A: Arbiter + Default>(trials: u64) -> MetricsReport {
    let mut failures = Vec::new();
    let mut hard_brakes = 0usize;
    let mut false_alarms = 0usize;
    let mut hard_transitions = 0usize;
    let mut cut_in_transitions = 0usize;
    let mut warn_latencies = Vec::new();
    let mut brake_latencies = Vec::new();

    for offset in 0..trials {
        let seed = HARD_SEED_BASE + offset;
        let run = decisions::<A>(Scenario::HardBrakingLead, seed);
        hard_transitions += transitions(&run);
        if ends_in_brake(&run) {
            hard_brakes += 1;
        } else {
            failures.push(Failure {
                scenario: "hard-braking",
                seed,
                reason: "no Brake in final 20 cycles",
            });
        }
    }

    for offset in 0..trials {
        let seed = EMPTY_SEED_BASE + offset;
        let run = decisions::<A>(Scenario::EmptyRoad, seed);
        if has_false_alarm(&run) {
            false_alarms += 1;
            failures.push(Failure {
                scenario: "empty-road",
                seed,
                reason: "Warn or Brake on empty road",
            });
        }
    }

    let truth = truth_ttc(Scenario::CutIn);
    let warn_crossing = first_below(&truth, WARN_TTC_S).expect("CutIn crosses warn threshold");
    let brake_crossing = first_below(&truth, BRAKE_TTC_S).expect("CutIn crosses brake threshold");
    for offset in 0..trials {
        let seed = CUT_IN_SEED_BASE + offset;
        let run = decisions::<A>(Scenario::CutIn, seed);
        cut_in_transitions += transitions(&run);
        match latency_at_or_after(&run, warn_crossing, Decision::Warn) {
            Some(latency) => {
                warn_latencies.push(latency);
                if latency > 3 {
                    failures.push(Failure {
                        scenario: "cut-in",
                        seed,
                        reason: "Warn response later than 3 cycles",
                    });
                }
            }
            None => failures.push(Failure {
                scenario: "cut-in",
                seed,
                reason: "no Warn response at or after truth crossing",
            }),
        }
        match latency_at_or_after(&run, brake_crossing, Decision::Brake) {
            Some(latency) => {
                brake_latencies.push(latency);
                if latency > 3 {
                    failures.push(Failure {
                        scenario: "cut-in",
                        seed,
                        reason: "Brake response later than 3 cycles",
                    });
                }
            }
            None => failures.push(Failure {
                scenario: "cut-in",
                seed,
                reason: "no Brake response at or after truth crossing",
            }),
        }
    }

    let warn_responses = warn_latencies.len();
    let brake_responses = brake_latencies.len();
    MetricsReport {
        trials,
        hard_brake_rate_pct: pct(hard_brakes, trials),
        empty_false_alarm_rate_pct: pct(false_alarms, trials),
        warn_latency_median_cycles: median(&mut warn_latencies),
        brake_latency_median_cycles: median(&mut brake_latencies),
        warn_response_coverage_pct: pct(warn_responses, trials),
        brake_response_coverage_pct: pct(brake_responses, trials),
        hard_chatter_transitions_per_run: hard_transitions as f64 / trials as f64,
        cut_in_chatter_transitions_per_run: cut_in_transitions as f64 / trials as f64,
        failures,
    }
}

impl MetricsReport {
    pub(crate) fn meets_targets(&self) -> bool {
        self.hard_brake_rate_pct >= 99.0
            && self.empty_false_alarm_rate_pct <= 2.0
            && self.warn_response_coverage_pct >= MIN_RESPONSE_COVERAGE_PCT
            && self.brake_response_coverage_pct >= MIN_RESPONSE_COVERAGE_PCT
            && self
                .warn_latency_median_cycles
                .is_some_and(|value| value <= 3.0)
            && self
                .brake_latency_median_cycles
                .is_some_and(|value| value <= 3.0)
    }

    pub(crate) fn print(&self) {
        println!("trials={}", self.trials);
        println!(
            "hard_brake_rate_pct={:.3} target=>=99",
            self.hard_brake_rate_pct
        );
        println!(
            "empty_false_alarm_rate_pct={:.3} target=<=2",
            self.empty_false_alarm_rate_pct
        );
        println!(
            "warn_response_coverage_pct={:.3} target=>=95",
            self.warn_response_coverage_pct
        );
        println!(
            "brake_response_coverage_pct={:.3} target=>=95",
            self.brake_response_coverage_pct
        );
        println!(
            "warn_latency_median_cycles={} target=<=3",
            display_median(self.warn_latency_median_cycles)
        );
        println!(
            "brake_latency_median_cycles={} target=<=3",
            display_median(self.brake_latency_median_cycles)
        );
        println!(
            "hard_chatter_transitions_per_run={:.3} diagnostic=lower-is-better",
            self.hard_chatter_transitions_per_run
        );
        println!(
            "cut_in_chatter_transitions_per_run={:.3} diagnostic=lower-is-better",
            self.cut_in_chatter_transitions_per_run
        );
        println!(
            "result={}",
            if self.meets_targets() {
                "PASS"
            } else {
                "NEEDS_REVISION"
            }
        );

        if !self.failures.is_empty() {
            println!("# replayable observations");
            for failure in &self.failures {
                println!(
                    "scenario={} seed={} reason={:?} replay=\"cargo run --release -p runner -- trace --scenario {} --seed {}\"",
                    failure.scenario,
                    failure.seed,
                    failure.reason,
                    failure.scenario,
                    failure.seed
                );
            }
        }
    }
}

fn display_median(value: Option<f64>) -> String {
    value.map_or_else(|| "n/a".to_string(), |value| format!("{value:.1}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brake_must_appear_in_the_final_second() {
        let mut values = vec![Decision::NoAction; 100];
        values[79] = Decision::Brake;
        assert!(!ends_in_brake(&values));
        values[80] = Decision::Brake;
        assert!(ends_in_brake(&values));
    }

    #[test]
    fn one_empty_road_warning_is_a_false_alarm() {
        let mut values = vec![Decision::NoAction; 100];
        values[5] = Decision::Warn;
        assert!(has_false_alarm(&values));
    }

    #[test]
    fn latency_search_starts_at_the_truth_crossing() {
        let mut values = vec![Decision::NoAction; 20];
        values[2] = Decision::Warn;
        values[12] = Decision::Warn;
        assert_eq!(latency_at_or_after(&values, 10, Decision::Warn), Some(2));
        assert_eq!(latency_at_or_after(&values, 13, Decision::Warn), None);
    }

    #[test]
    fn median_handles_even_odd_and_no_responses() {
        assert_eq!(median(&mut []), None);
        assert_eq!(median(&mut [3]), Some(3.0));
        assert_eq!(median(&mut [4, 2]), Some(3.0));
    }
}
