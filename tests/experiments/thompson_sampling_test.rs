#[test]
fn test_thompson_sampling_selection() {
    let arms = vec!["scenario_a".to_string(), "scenario_b".to_string()];
    let ts = ThompsonSampling::new(arms);

    let selected = ts.select_arm().unwrap();
    assert!(selected == "scenario_a" || selected == "scenario_b");
}

#[test]
fn test_thompson_sampling_update() {
    let arms = vec!["scenario_a".to_string(), "scenario_b".to_string()];
    let mut ts = ThompsonSampling::new(arms);

    // Simulate 100 pulls with scenario_a always winning
    for _ in 0..100 {
        ts.update("scenario_a", 1.0).unwrap();
        ts.update("scenario_b", 0.0).unwrap();
    }

    let stats = ts.get_stats();
    let a_stats = stats.get("scenario_a").unwrap();
    let b_stats = stats.get("scenario_b").unwrap();

    // scenario_a should have higher mean
    assert!(a_stats.mean > b_stats.mean);
}