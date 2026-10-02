use crate::config::GroupConfig;

pub fn random_hex_suffix() -> String {
    format!(
        "{:016x}",
        getrandom::u64().expect("Failed to generate random u64")
    )
}

pub struct RunnerNames {
    pub github_name: String,
    pub hostname: String,
}

pub fn generate_runner_name(config: &GroupConfig) -> RunnerNames {
    let suffix = random_hex_suffix();
    RunnerNames {
        github_name: format!("{}-{suffix}", config.runner_name_prefix),
        hostname: format!("{}-{suffix}", config.hostname_prefix),
    }
}
