//! Prepared scenario runner for the three-hour Project C route.
//!
//! The runner owns mechanical trial loops and metric definitions. Students own
//! `guardian::ParticipantArbiter`, its policy, and the evidence that the policy
//! supports. Read `metrics.rs` before trusting the numbers.

mod metrics;

use guardian::{Arbiter, ParticipantArbiter};
use sensor_sim::{Scenario, Sim, SimConfig};

fn parse_scenario(value: &str) -> Result<Scenario, String> {
    match value {
        "hard-braking" => Ok(Scenario::HardBrakingLead),
        "cut-in" => Ok(Scenario::CutIn),
        "constant-lead" => Ok(Scenario::ConstantLead),
        "empty-road" => Ok(Scenario::EmptyRoad),
        _ => Err(format!("unknown scenario: {value}")),
    }
}

fn trace(mut args: impl Iterator<Item = String>) -> Result<(), String> {
    let mut scenario = Scenario::HardBrakingLead;
    let mut seed = 3_007u64;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--scenario" => {
                scenario = parse_scenario(
                    &args
                        .next()
                        .ok_or_else(|| "--scenario takes a name".to_string())?,
                )?;
            }
            "--seed" => {
                seed = args
                    .next()
                    .ok_or_else(|| "--seed takes an integer".to_string())?
                    .parse()
                    .map_err(|_| "--seed takes an integer".to_string())?;
            }
            _ => return Err(format!("unknown trace argument: {arg}")),
        }
    }

    let sim = Sim::new(
        scenario,
        SimConfig {
            seed: Some(seed),
            ..SimConfig::default()
        },
    );
    let mut arbiter = make_default::<ParticipantArbiter>();

    println!("# scenario: {scenario:?}  seed: {seed}");
    println!("# cycle  time_s  radar  camera  min_range_m  decision");
    for (cycle, reports) in sim.enumerate() {
        let decision = arbiter.decide(&reports);
        let min_range = reports
            .iter()
            .map(|report| report.range_m)
            .fold(f64::INFINITY, f64::min);
        let radar = reports
            .iter()
            .filter(|report| report.sensor == sensor_sim::Sensor::Radar)
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
    Ok(())
}

fn make_default<T: Default>() -> T {
    T::default()
}

fn measure(mut args: impl Iterator<Item = String>) -> Result<bool, String> {
    let mut trials = 200u64;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--trials" => {
                trials = args
                    .next()
                    .ok_or_else(|| "--trials takes a positive integer".to_string())?
                    .parse()
                    .map_err(|_| "--trials takes a positive integer".to_string())?;
                if trials == 0 {
                    return Err("--trials must be greater than zero".to_string());
                }
            }
            _ => return Err(format!("unknown metrics argument: {arg}")),
        }
    }

    let report = metrics::evaluate::<ParticipantArbiter>(trials);
    report.print();
    Ok(report.meets_targets())
}

fn usage() {
    eprintln!(
        "usage:\n  runner trace [--scenario hard-braking|cut-in|constant-lead|empty-road] [--seed N]\n  runner metrics [--trials N]"
    );
}

fn main() {
    let mut args = std::env::args().skip(1);
    let result = match args.next().as_deref() {
        Some("trace") => trace(args).map(|()| true),
        Some("metrics") => measure(args),
        _ => {
            usage();
            std::process::exit(2);
        }
    };

    match result {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(message) => {
            eprintln!("error: {message}");
            usage();
            std::process::exit(2);
        }
    }
}
