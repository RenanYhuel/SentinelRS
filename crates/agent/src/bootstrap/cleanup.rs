pub fn scrub_token_from_env() {
    std::env::remove_var("BOOTSTRAP_TOKEN");
    tracing::debug!("bootstrap token scrubbed from environment");
}
