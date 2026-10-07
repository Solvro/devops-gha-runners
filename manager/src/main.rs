use cloneable_errors::{ErrorContext, ResContext};
use tokio::io::AsyncReadExt;

use crate::utils::{create_runner, generate_runner_name};

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

    let mut runner = create_runner(
        &config.program,
        &config.groups[0],
        &runner_name.hostname,
        &resp.encoded_jit_config,
    )
    .await
    .context("Failed to spawn the runner")?;
    let mut buf = [0u8; 256];
    let bytes_read = runner
        .ready_signal_pipe
        .read(&mut buf)
        .await
        .context("Failed to read from runner signal pipe")?;
    let out = str::from_utf8(&buf[0..bytes_read]).context("Got non-utf8 bytes from proc")?;
    println!("runner output: {out}");
    runner
        .process
        .wait()
        .await
        .context("Error while waiting for proc")?;

    Ok(())
}
