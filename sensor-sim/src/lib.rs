//! sensor-sim: scripted, noisy track reports at 20 Hz for `guardian`.
//!
//! One `Sim` iterates cycles; each cycle yields the `Vec<TrackReport>`
//! observed in that 50 ms window. Reports are noisy by design: Gaussian
//! noise on range and range-rate, dropouts lasting 1–3 cycles, and spurious
//! low-confidence ghost tracks. Two sensor channels observe the same
//! world: radar (good range rate, multipath ghosts) and camera (noisier
//! rate, glare ghosts). Their dropouts and ghosts are decorrelated; a
//! cycle can carry reports from both, one, or neither channel.
//!
//! For deterministic golden tests, use [`SimConfig::noiseless`] and a fixed
//! seed. For statistical verdicts, vary the seed or use `seed: None`.
//! Seeding inside a probabilistic test defeats its purpose.

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Simulation cycle period: 20 Hz.
pub const CYCLE_DT_S: f64 = 0.05;

/// Which perception channel produced a report. Both observe the same
/// world; their noise, dropouts, and spurious detections differ and are
/// decorrelated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sensor {
    Radar,
    Camera,
}

/// One perceived object in one cycle, from one channel.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackReport {
    /// The channel that produced this report.
    pub sensor: Sensor,
    /// Track id. Ids are reused over time and shared between real
    /// objects and spurious detections; nothing in the id
    /// distinguishes them.
    pub object_id: u32,
    /// Range to object, meters. Noisy.
    pub range_m: f64,
    /// Rate of change of range, m/s. Negative = closing. Noisy.
    pub range_rate_mps: f64,
    /// Perception confidence in [0, 1].
    pub confidence: f64,
}

/// Parameters for a generic scripted lead vehicle, expressed in closing
/// terms: the sim tracks range and closing speed, not absolute speeds.
/// From `accel_start_s` on, closing speed grows by `closing_accel_mps2`
/// per second (a decelerating lead) until it reaches `closing_max_mps`
/// (the lead has stopped, or the speed difference has peaked). Protocol
/// cells map onto this by unit conversion; that mapping is your work.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScriptedLead {
    /// Initial range to the lead, meters (a protocol headway).
    pub initial_range_m: f64,
    /// Initial closing speed, m/s. Zero when both vehicles hold the
    /// same speed; positive when the subject approaches.
    pub initial_closing_mps: f64,
    /// Time the lead starts changing speed, seconds.
    pub accel_start_s: f64,
    /// Growth of closing speed from `accel_start_s`, m/s^2. A braking
    /// lead makes this positive. Applied as constant acceleration; a
    /// protocol ramp-in tolerance is a simplification you should note.
    pub closing_accel_mps2: f64,
    /// Cap on closing speed, m/s. A stopped lead caps closing at the
    /// subject's speed.
    pub closing_max_mps: f64,
    /// Lead confidence as perceived, constant. 0.95 matches the named
    /// scenarios' established leads.
    pub confidence: f64,
}

/// Scripted scenarios. Ground truth is deterministic; observation is not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scenario {
    /// Lead vehicle ahead at ~80 m, slowly closing (~2 m/s). Should end
    /// without intervention.
    ConstantLead,
    /// Lead at ~50 m closing ~3 m/s; at t = 2 s the lead brakes hard and
    /// closing speed ramps to ~15 m/s. Collision imminent unless braking.
    HardBrakingLead,
    /// Empty road until t = 1.5 s, then a vehicle cuts in at ~25 m closing
    /// ~8 m/s; its confidence ramps 0.2 → 0.9 over ~0.5 s.
    CutIn,
    /// No true objects at all. Anything reported is a ghost.
    EmptyRoad,
    /// A single lead following a caller-supplied script. The mechanism
    /// for building protocol test cells; see the SPEC's protocol
    /// alignment section.
    Scripted(ScriptedLead),
}

/// Simulator configuration.
#[derive(Clone, Debug)]
pub struct SimConfig {
    /// Std-dev of Gaussian noise added to range, meters.
    pub range_noise_std_m: f64,
    /// Std-dev of Gaussian noise added to range-rate, m/s.
    pub rate_noise_std_mps: f64,
    /// Per-cycle probability that a visible true track begins a dropout
    /// (lasting 1–3 cycles).
    pub dropout_prob: f64,
    /// Per-cycle probability that the camera loses a visible true track
    /// (decorrelated from radar dropouts).
    pub camera_dropout_prob: f64,
    /// Camera range noise, meters (worse than radar).
    pub camera_range_noise_std_m: f64,
    /// Camera range-rate noise, m/s (much worse than radar).
    pub camera_rate_noise_std_mps: f64,
    /// Per-cycle probability of a spurious camera detection (glare,
    /// texture); decorrelated from radar multipath ghosts.
    pub camera_ghost_prob: f64,
    /// Per-cycle probability that a ghost track spawns (lasting 1–2 cycles).
    pub ghost_prob: f64,
    /// Number of cycles to simulate (200 = 10 s).
    pub cycles: usize,
    /// RNG seed. `None` seeds from entropy, so every run differs.
    pub seed: Option<u64>,
}

impl Default for SimConfig {
    fn default() -> Self {
        SimConfig {
            range_noise_std_m: 0.8,
            rate_noise_std_mps: 0.6,
            dropout_prob: 0.02,
            camera_dropout_prob: 0.03,
            camera_range_noise_std_m: 1.6,
            camera_rate_noise_std_mps: 1.2,
            camera_ghost_prob: 0.012,
            ghost_prob: 0.03,
            cycles: 200,
            seed: None,
        }
    }
}

impl SimConfig {
    /// Noise, dropouts, and ghosts all off, for deterministic golden tests
    /// of the decision core. The seed is irrelevant once noise is off, but
    /// determinism should be explicit.
    pub fn noiseless() -> Self {
        SimConfig {
            range_noise_std_m: 0.0,
            rate_noise_std_mps: 0.0,
            dropout_prob: 0.0,
            camera_dropout_prob: 0.0,
            camera_range_noise_std_m: 0.0,
            camera_rate_noise_std_mps: 0.0,
            camera_ghost_prob: 0.0,
            ghost_prob: 0.0,
            cycles: 200,
            seed: Some(0),
        }
    }
}

struct TrueTrack {
    id: u32,
    range_m: f64,
    closing_mps: f64, // positive = closing
    confidence: f64,
    dropout_left: u32,
    visible: bool,
    cam_dropout_left: u32,
    cam_visible: bool,
}

struct Ghost {
    sensor: Sensor,
    id: u32,
    range_m: f64,
    rate_mps: f64,
    confidence: f64,
    life_left: u32,
}

/// Begin a dropout: the start cycle is the first hidden cycle, so a
/// requested length of N hides the track for exactly N cycles.
fn start_dropout_on(track: &mut TrueTrack, len: u32) {
    track.dropout_left = len - 1;
    track.visible = false;
}

/// The simulator. Iterate it: one `Vec<TrackReport>` per 50 ms cycle.
pub struct Sim {
    scenario: Scenario,
    cfg: SimConfig,
    rng: StdRng,
    cycle: usize,
    tracks: Vec<TrueTrack>,
    ghosts: Vec<Ghost>,
}

impl Sim {
    pub fn new(scenario: Scenario, cfg: SimConfig) -> Self {
        let rng = match cfg.seed {
            Some(s) => StdRng::seed_from_u64(s),
            None => StdRng::from_entropy(),
        };
        let tracks = match scenario {
            Scenario::ConstantLead => vec![TrueTrack {
                id: 1,
                range_m: 80.0,
                closing_mps: 2.0,
                confidence: 0.92,
                dropout_left: 0,
                visible: true,
                cam_dropout_left: 0,
                cam_visible: true,
            }],
            Scenario::HardBrakingLead => vec![TrueTrack {
                id: 1,
                range_m: 50.0,
                closing_mps: 3.0,
                confidence: 0.95,
                dropout_left: 0,
                visible: true,
                cam_dropout_left: 0,
                cam_visible: true,
            }],
            Scenario::CutIn => Vec::new(), // appears at t = 1.5 s
            Scenario::EmptyRoad => Vec::new(),
            Scenario::Scripted(s) => vec![TrueTrack {
                id: 1,
                range_m: s.initial_range_m,
                closing_mps: s.initial_closing_mps,
                confidence: s.confidence,
                dropout_left: 0,
                visible: true,
                cam_dropout_left: 0,
                cam_visible: true,
            }],
        };
        Sim {
            scenario,
            cfg,
            rng,
            cycle: 0,
            tracks,
            ghosts: Vec::new(),
        }
    }

    /// Current simulation time, seconds.
    pub fn time_s(&self) -> f64 {
        self.cycle as f64 * CYCLE_DT_S
    }

    fn gaussian(&mut self, std: f64) -> f64 {
        if std == 0.0 {
            return 0.0;
        }
        // Box–Muller.
        let u1: f64 = self.rng.gen_range(f64::EPSILON..1.0);
        let u2: f64 = self.rng.gen::<f64>();
        std * (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }

    fn advance_truth(&mut self) {
        let t = self.time_s();

        // Scenario scripting.
        match self.scenario {
            Scenario::HardBrakingLead => {
                if let Some(lead) = self.tracks.iter_mut().find(|tr| tr.id == 1) {
                    if t >= 2.0 && lead.closing_mps < 15.0 {
                        lead.closing_mps = (lead.closing_mps + 6.0 * CYCLE_DT_S).min(15.0);
                    }
                }
            }
            Scenario::CutIn => {
                if t >= 1.5 && self.tracks.is_empty() {
                    self.tracks.push(TrueTrack {
                        id: 2,
                        range_m: 25.0,
                        closing_mps: 8.0,
                        confidence: 0.2, // ramps up below
                        dropout_left: 0,
                        visible: true,
                        cam_dropout_left: 0,
                        cam_visible: true,
                    });
                }
                if let Some(tr) = self.tracks.iter_mut().find(|tr| tr.id == 2) {
                    tr.confidence = (tr.confidence + 1.4 * CYCLE_DT_S).min(0.9);
                }
            }
            Scenario::ConstantLead | Scenario::EmptyRoad => {}
            Scenario::Scripted(s) => {
                if t >= s.accel_start_s {
                    if let Some(lead) = self.tracks.iter_mut().find(|tr| tr.id == 1) {
                        lead.closing_mps = (lead.closing_mps + s.closing_accel_mps2 * CYCLE_DT_S)
                            .min(s.closing_max_mps);
                    }
                }
            }
        }

        // Kinematics + dropout state.
        let dropout_prob = self.cfg.dropout_prob;
        let cam_dropout_prob = self.cfg.camera_dropout_prob;
        let mut rolls: Vec<(bool, u32, bool, u32)> = Vec::with_capacity(self.tracks.len());
        for _ in 0..self.tracks.len() {
            let start = self.rng.gen::<f64>() < dropout_prob;
            let len = self.rng.gen_range(1..=3u32);
            let cam_start = self.rng.gen::<f64>() < cam_dropout_prob;
            let cam_len = self.rng.gen_range(1..=3u32);
            rolls.push((start, len, cam_start, cam_len));
        }
        for (track, (start_dropout, len, cam_start, cam_len)) in self.tracks.iter_mut().zip(rolls) {
            track.range_m = (track.range_m - track.closing_mps * CYCLE_DT_S).max(0.0);
            if track.dropout_left > 0 {
                track.dropout_left -= 1;
                track.visible = false;
            } else if start_dropout {
                start_dropout_on(track, len);
            } else {
                track.visible = true;
            }
            // The camera loses tracks on its own schedule: fog, glare,
            // and occlusion do not consult the radar.
            if track.cam_dropout_left > 0 {
                track.cam_dropout_left -= 1;
                track.cam_visible = false;
            } else if cam_start {
                track.cam_dropout_left = cam_len - 1;
                track.cam_visible = false;
            } else {
                track.cam_visible = true;
            }
        }
        // Objects that reached us stay pinned at 0 range (scenario over,
        // effectively); guardian should long since have decided.
    }

    fn spawn_and_age_ghosts(&mut self) {
        // Radar multipath and camera glare are different physics; each
        // channel rolls its own spurious detections. Ids come from the
        // same small space real tracks use, so no id heuristic can
        // separate spurious from real on either channel.
        for (sensor, prob) in [
            (Sensor::Radar, self.cfg.ghost_prob),
            (Sensor::Camera, self.cfg.camera_ghost_prob),
        ] {
            if self.rng.gen::<f64>() < prob {
                let id = self.rng.gen_range(1..=8);
                let range = self.rng.gen_range(5.0..120.0);
                let rate = self.rng.gen_range(-10.0..10.0);
                let confidence = self.rng.gen_range(0.05..0.45);
                let life = self.rng.gen_range(1..=2u32);
                self.ghosts.push(Ghost {
                    sensor,
                    id,
                    range_m: range,
                    rate_mps: rate,
                    confidence,
                    life_left: life,
                });
            }
        }
    }

    fn observe(&mut self) -> Vec<TrackReport> {
        let mut out = Vec::new();
        let range_std = self.cfg.range_noise_std_m;
        let rate_std = self.cfg.rate_noise_std_mps;
        let cam_range_std = self.cfg.camera_range_noise_std_m;
        let cam_rate_std = self.cfg.camera_rate_noise_std_mps;
        for i in 0..self.tracks.len() {
            let (id, range, closing, conf, radar_sees, camera_sees) = {
                let t = &self.tracks[i];
                (
                    t.id,
                    t.range_m,
                    t.closing_mps,
                    t.confidence,
                    t.visible,
                    t.cam_visible,
                )
            };
            if radar_sees {
                let nr = self.gaussian(range_std);
                let nv = self.gaussian(rate_std);
                out.push(TrackReport {
                    sensor: Sensor::Radar,
                    object_id: id,
                    range_m: (range + nr).max(0.0),
                    range_rate_mps: -closing + nv,
                    confidence: conf,
                });
            }
            if camera_sees {
                let nr = self.gaussian(cam_range_std);
                let nv = self.gaussian(cam_rate_std);
                out.push(TrackReport {
                    sensor: Sensor::Camera,
                    object_id: id,
                    range_m: (range + nr).max(0.0),
                    range_rate_mps: -closing + nv,
                    confidence: conf,
                });
            }
        }
        for i in 0..self.ghosts.len() {
            let (sensor, id, range, rate, conf) = {
                let g = &self.ghosts[i];
                (g.sensor, g.id, g.range_m, g.rate_mps, g.confidence)
            };
            let nr = match sensor {
                Sensor::Radar => self.gaussian(range_std),
                Sensor::Camera => self.gaussian(cam_range_std),
            };
            out.push(TrackReport {
                sensor,
                object_id: id,
                range_m: (range + nr).max(0.0),
                range_rate_mps: rate,
                confidence: conf,
            });
        }
        out
    }
}

impl Iterator for Sim {
    type Item = Vec<TrackReport>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.cycle >= self.cfg.cycles {
            return None;
        }
        self.advance_truth();
        self.spawn_and_age_ghosts();
        let reports = self.observe();
        // Age ghosts after they have been observed, so life N means
        // exactly N observable cycles.
        for g in &mut self.ghosts {
            g.life_left = g.life_left.saturating_sub(1);
        }
        self.ghosts.retain(|g| g.life_left > 0);
        self.cycle += 1;
        Some(reports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeded_runs_are_identical() {
        let cfg = SimConfig {
            seed: Some(1234),
            ..SimConfig::default()
        };
        let a: Vec<_> = Sim::new(Scenario::HardBrakingLead, cfg.clone()).collect();
        let b: Vec<_> = Sim::new(Scenario::HardBrakingLead, cfg).collect();
        assert_eq!(a, b);
    }

    #[test]
    fn empty_road_reports_only_ghosts() {
        let sim = Sim::new(Scenario::EmptyRoad, SimConfig::default());
        for cycle in sim {
            for report in cycle {
                assert!((0.05..0.45).contains(&report.confidence));
                assert!((1..=8).contains(&report.object_id));
            }
        }
    }

    #[test]
    fn ghost_life_n_is_observed_exactly_n_cycles() {
        for life in 1..=2u32 {
            let mut cfg = SimConfig::noiseless();
            cfg.cycles = 10;
            let mut sim = Sim::new(Scenario::EmptyRoad, cfg);
            sim.ghosts.push(Ghost {
                sensor: Sensor::Radar,
                id: 7,
                range_m: 30.0,
                rate_mps: -5.0,
                confidence: 0.2,
                life_left: life,
            });
            let seen = sim
                .flat_map(|c| c.into_iter())
                .filter(|r| r.object_id == 7)
                .count() as u32;
            assert_eq!(seen, life);
        }
    }

    #[test]
    fn in_progress_dropout_hides_exactly_remaining_cycles() {
        let mut cfg = SimConfig::noiseless();
        cfg.cycles = 6;
        let mut sim = Sim::new(Scenario::ConstantLead, cfg);
        sim.tracks[0].dropout_left = 2;
        sim.tracks[0].visible = false;
        let visibility: Vec<bool> = (&mut sim)
            .map(|c| {
                c.iter()
                    .any(|r| r.sensor == Sensor::Radar && r.object_id == sim_true_id())
            })
            .collect();
        assert_eq!(&visibility[..3], &[false, false, true]);
    }

    fn sim_true_id() -> u32 {
        1
    }

    #[test]
    fn requested_dropout_lengths_hide_exactly_that_many_cycles() {
        for len in 1..=3u32 {
            let mut cfg = SimConfig::noiseless();
            cfg.cycles = 8;
            let mut sim = Sim::new(Scenario::ConstantLead, cfg);
            // The start cycle: the dropout begins mid-advance and this
            // cycle's observation already misses the track.
            start_dropout_on(&mut sim.tracks[0], len);
            assert!(sim
                .observe()
                .iter()
                .all(|r| r.sensor != Sensor::Radar || r.object_id != 1));
            // Continuation cycles through the normal iterator path.
            let more = (&mut sim)
                .map(|c| {
                    c.iter()
                        .all(|r| r.sensor != Sensor::Radar || r.object_id != 1)
                })
                .take_while(|h| *h)
                .count() as u32;
            assert_eq!(1 + more, len);
        }
    }

    #[test]
    fn scripted_lead_follows_its_script() {
        // A braking-lead shape: same speeds (closing 0) from 12 m, then
        // closing grows at 6 m/s^2 up to 13.89 m/s from t = 1 s.
        let s = ScriptedLead {
            initial_range_m: 12.0,
            initial_closing_mps: 0.0,
            accel_start_s: 1.0,
            closing_accel_mps2: 6.0,
            closing_max_mps: 13.89,
            confidence: 0.95,
        };
        let mut cfg = SimConfig::noiseless();
        cfg.cycles = 100;
        let sim = Sim::new(Scenario::Scripted(s), cfg);
        let mut prev_range = f64::INFINITY;
        let mut range_at_1s = None;
        let mut max_closing_seen: f64 = 0.0;
        for (cycle, reports) in sim.enumerate() {
            let r = reports
                .iter()
                .find(|r| r.sensor == Sensor::Radar && r.object_id == 1)
                .expect("noiseless scripted lead is always visible");
            let t = cycle as f64 * CYCLE_DT_S;
            if t < 1.0 - 1e-9 {
                // Before the script starts, range holds at 12 m.
                assert!((r.range_m - 12.0).abs() < 1e-6, "range moved early");
                range_at_1s = Some(r.range_m);
            } else {
                assert!(r.range_m <= prev_range, "range must close");
            }
            max_closing_seen = max_closing_seen.max(-r.range_rate_mps);
            prev_range = r.range_m;
        }
        assert!(range_at_1s.is_some());
        // Closing speed caps at the scripted maximum.
        assert!(max_closing_seen <= 13.89 + 1e-6);
        assert!(max_closing_seen > 13.0, "cap was never approached");
    }

    #[test]
    fn camera_and_radar_drop_out_independently() {
        let cfg = SimConfig {
            cycles: 3000,
            ghost_prob: 0.0,
            camera_ghost_prob: 0.0,
            seed: Some(17),
            ..SimConfig::default()
        };
        let mut radar_only = 0u32;
        let mut camera_only = 0u32;
        let mut both = 0u32;
        for c in Sim::new(Scenario::ConstantLead, cfg) {
            let radar = c
                .iter()
                .any(|r| r.sensor == Sensor::Radar && r.object_id == 1);
            let camera = c
                .iter()
                .any(|r| r.sensor == Sensor::Camera && r.object_id == 1);
            match (radar, camera) {
                (true, false) => radar_only += 1,
                (false, true) => camera_only += 1,
                (true, true) => both += 1,
                (false, false) => {}
            }
        }
        // Decorrelation: each channel sometimes covers the other's gap,
        // and most cycles both see the object.
        assert!(radar_only > 10, "camera dropouts never solo: {radar_only}");
        assert!(camera_only > 10, "radar dropouts never solo: {camera_only}");
        assert!(both > 2000);
    }

    #[test]
    fn seeded_dropout_gaps_stay_within_three_cycles() {
        let cfg = SimConfig {
            cycles: 3000,
            ghost_prob: 0.0,
            seed: Some(11),
            ..SimConfig::default()
        };
        // A new dropout may begin the cycle after one ends, merging two
        // gaps, so single dropouts bound gaps at 3 and one merge at 6.
        let mut gap = 0u32;
        let mut short_gaps = 0u32;
        for c in Sim::new(Scenario::ConstantLead, cfg) {
            if c.iter()
                .any(|r| r.sensor == Sensor::Radar && r.object_id == 1)
            {
                assert!(gap <= 6, "dropout gap of {gap} cycles");
                if (1..=3).contains(&gap) {
                    short_gaps += 1;
                }
                gap = 0;
            } else {
                gap += 1;
            }
        }
        assert!(short_gaps > 10);
    }

    #[test]
    fn noiseless_hard_brake_closes_range() {
        let mut cfg = SimConfig::noiseless();
        cfg.cycles = 200;
        let cycles: Vec<_> = Sim::new(Scenario::HardBrakingLead, cfg).collect();
        let first = cycles
            .first()
            .and_then(|c| c.first())
            .expect("lead visible");
        let last = cycles
            .iter()
            .rev()
            .find_map(|c| c.first())
            .expect("lead visible at end");
        assert!(last.range_m < first.range_m);
        assert!(last.range_m < 5.0, "lead should be on top of us by t=10s");
    }

    #[test]
    fn cut_in_confidence_ramps() {
        let cfg = SimConfig::noiseless();
        let mut early_conf = None;
        let mut late_conf = None;
        for (i, cycle) in Sim::new(Scenario::CutIn, cfg).enumerate() {
            if let Some(r) = cycle.iter().find(|r| r.object_id == 2) {
                if early_conf.is_none() {
                    early_conf = Some(r.confidence);
                }
                if i > 60 {
                    late_conf = Some(r.confidence);
                }
            }
        }
        assert!(early_conf.expect("cut-in appears") < 0.3);
        assert!(late_conf.expect("cut-in persists") > 0.8);
    }
}
