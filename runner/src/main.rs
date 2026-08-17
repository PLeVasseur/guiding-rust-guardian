//! Scenario runner scaffold.
//!
//! Compiles after the participant-owned `ParticipantArbiter` skeleton exists:
//! it drives `sensor-sim` and prints a per-cycle trace. Wire your arbiter in at
//! the marked line, then grow the metrics section toward the acceptance table. The
//! table is defined over many runs; a single run shows very little.

use guardian::{Arbiter, Decision};
use sensor_sim::{Scenario, Sim, SimConfig};

fn parse_scenario(s: &str) -> Option<Scenario> {
    match s {
        "hard-braking" => Some(Scenario::HardBrakingLead),
        "cut-in" => Some(Scenario::CutIn),
        "constant-lead" => Some(Scenario::ConstantLead),
        "empty-road" => Some(Scenario::EmptyRoad),
        _ => None,
    }
}

fn main() {
    // trace [--scenario NAME] [--seed N]   deterministic single run
    // metrics --trials N                   participant work: see below
    let mut args = std::env::args().skip(1);
    let mut scenario = Scenario::HardBrakingLead;
    let mut seed: Option<u64> = None;
    let mut mode_metrics = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "trace" => {}
            "metrics" => mode_metrics = true,
            "--scenario" => {
                let v = args.next().expect("--scenario takes a name");
                scenario = parse_scenario(&v).expect("unknown scenario");
            }
            "--seed" => {
                seed = Some(
                    args.next()
                        .and_then(|v| v.parse().ok())
                        .expect("--seed takes an integer"),
                );
            }
            "--trials" => {
                let _ = args.next();
            }
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }
    if mode_metrics {
        // Participant work. The acceptance table is defined over many
        // runs; your metrics must print, for every failing run, the
        // scenario name and seed, so any failure replays exactly with:
        //   runner trace --scenario NAME --seed N
        eprintln!("metrics: not implemented yet (this is your work)");
        std::process::exit(2);
    }
    let cfg = SimConfig {
        seed,
        ..SimConfig::default()
    };
    let sim = Sim::new(scenario, cfg);

    println!(
        "# scenario: {scenario:?}  seed: {seed:?}  (cycle dt = {} s)",
        sensor_sim::CYCLE_DT_S
    );
    println!("# cycle  time_s  radar  camera  min_range_m  decision");

    let mut final_decision = Decision::NoAction;
    // Keep this adapter stable so the same runner and independent evaluator can
    // exercise every participant design.
    let mut arbiter = guardian::ParticipantArbiter::default();
    for (cycle, reports) in sim.enumerate() {
        let min_range = reports
            .iter()
            .map(|r| r.range_m)
            .fold(f64::INFINITY, f64::min);

        let decision = arbiter.decide(&reports);

        final_decision = final_decision.max(decision);
        let radar = reports
            .iter()
            .filter(|r| r.sensor == sensor_sim::Sensor::Radar)
            .count();
        let camera = reports.len() - radar;
        println!(
            "{cycle:>5}  {:>6.2}  {radar:>5}  {camera:>6}  {:>11.2}  {decision:?}",
            cycle as f64 * sensor_sim::CYCLE_DT_S,
            if min_range.is_finite() {
                min_range
            } else {
                f64::NAN
            },
        );
    }
    println!("# most severe decision this run: {final_decision:?}");
    println!("# (metrics are computed over many runs; see the SPEC.md acceptance table)");
}
