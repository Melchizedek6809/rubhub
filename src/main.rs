fn main() {
    let start = std::time::Instant::now();

    // We do this two-step approach to make it easier to do integration tests.
    let config = rubhub::AppConfig::new().expect("Error creating AppConfig");
    rubhub::run_multi_thread(config, start)
}
