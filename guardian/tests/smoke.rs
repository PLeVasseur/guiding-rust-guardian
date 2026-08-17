//! Proves the test rig runs. The suite itself is your work: which
//! invariants, which strategies, how many samples, and why, per the
//! acceptance table.

use guardian::Decision;

#[test]
fn decision_severity_is_ordered() {
    assert!(Decision::NoAction < Decision::Warn);
    assert!(Decision::Warn < Decision::Brake);
}
