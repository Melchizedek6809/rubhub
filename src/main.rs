fn main() {
    let start = std::time::Instant::now();

    #[cfg(debug_assertions)]
    if dotenvy::dotenv().is_err() {
        println!("No .env found, using dev defaults");
    }

    // We do this two-step approach to make it easier to do integration tests.
    let config = rubhub::AppConfig::new().expect("Error creating AppConfig");
    let state = config
        .build(start)
        .expect("Error creating GlobalState from AppConfig");
    
    rubhub::run_single_thread(state)
}
