use cloneable_errors::{ErrorContext, ResContext};

use crate::utils::generate_runner_name;

mod config;
mod utils;

#[tokio::main]
async fn main() -> Result<(), ErrorContext> {
    tracing_subscriber::fmt::init();
    let config = config::Config::get().context("Failed to load config file")?;
    let client = config
        .app
        .build_client()
        .context("Failed to build client from config file data")?;

    let runner_name = generate_runner_name(&config.groups[0]);
    let resp = client
        .actions()
        .create_org_jit_runner_config(
            &*config.app.org_name,
            runner_name.github_name,
            config.groups[0].group_id,
            config.groups[0]
                .labels
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
        )
        .work_folder("/runner")
        .send()
        .await
        .context("Failed to create jit runner")?;

    println!("{}", resp.encoded_jit_config);

    Ok(())
}
