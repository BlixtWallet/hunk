use hunk_sleep_inhibitor::SleepInhibitor;

#[test]
fn tracks_turn_state_while_disabled() {
    let mut inhibitor = SleepInhibitor::new(false);

    inhibitor.set_turn_running(true);

    assert!(!inhibitor.enabled());
    assert!(inhibitor.is_turn_running());
}

#[test]
fn toggles_enabled_state_without_panicking() {
    let mut inhibitor = SleepInhibitor::new(false);

    inhibitor.set_turn_running(true);
    inhibitor.set_enabled(true);
    inhibitor.set_enabled(false);
    inhibitor.set_turn_running(false);

    assert!(!inhibitor.enabled());
    assert!(!inhibitor.is_turn_running());
}

#[test]
fn repeated_running_updates_are_allowed() {
    let mut inhibitor = SleepInhibitor::new(true);

    inhibitor.set_turn_running(true);
    inhibitor.set_turn_running(true);
    inhibitor.set_turn_running(true);
    inhibitor.set_turn_running(false);

    assert!(inhibitor.enabled());
    assert!(!inhibitor.is_turn_running());
}
