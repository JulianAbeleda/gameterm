pub fn gameterm_version() -> &'static str {
    // See build.rs
    env!("GAMETERM_CI_TAG")
}

pub fn gameterm_target_triple() -> &'static str {
    // See build.rs
    env!("GAMETERM_TARGET_TRIPLE")
}
