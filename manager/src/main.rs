use std::time::{Duration, Instant};

use async_signal::{Signal, Signals};
use cloneable_errors::{ErrorContext, ResContext};
use futures_lite::StreamExt;
use octocrab::{Octocrab, models::RunnerGroupId};
use tokio::{io::AsyncReadExt, select, time::sleep};
use tracing::{error, info, warn};

use crate::{
    config::GroupStatus,
    utils::{create_runner, generate_runner_name, kill_runner},
};

mod config;
mod utils;

/// if a runner finishes (or fails to start) earlier than this duration,
/// add an artificial wait before actually marking the runner as finished
const MIN_RUNNER_LIFETIME: Duration = Duration::from_secs(15);

async fn artificial_delay(start_time: Instant) {
    let elapsed = start_time.elapsed();
    if let Some(to_wait) = MIN_RUNNER_LIFETIME.checked_sub(elapsed) {
        sleep(to_wait).await;
    }
}

/// create and manage a runner lifetime
///
/// the caller should increment the runner total and idle counts before calling this function
#[allow(clippy::too_many_lines)]
async fn spawn_runner(
    client: &Octocrab,
    config: &config::Config,
    group: &config::GroupConfig,
    mut guard: utils::CounterGuard,
) {
    let start_time = Instant::now();
    let runner_name = generate_runner_name(group);

    info!("Creating runner {}", runner_name.hostname);
    // get a config for the runner from github
    let resp = match client
        .actions()
        .create_org_jit_runner_config(
            &*config.app.org_name,
            runner_name.github_name,
            group.group_id,
            group
                .labels
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
        )
        .work_folder("/runner")
        .send()
        .await
    {
        Ok(v) => v,
        Err(e) => {
            error!(
                "Failed to create JIT runner config for runner {}: {e:?}",
                runner_name.hostname
            );
            // treat the runner as busy, but not finished yet
            group.status.send_modify(|status| {
                status.idle -= 1;
            });
            // wait like 15s so that we don't end up spamming the github API
            artificial_delay(start_time).await;
            // mark it as done
            group.status.send_modify(|status| {
                status.total -= 1;
            });
            return;
        }
    };

    info!("Runner {} created in GitHub", runner_name.hostname);

    // create the runner container
    let mut runner = match create_runner(
        &config.program,
        group,
        &runner_name.hostname,
        &resp.encoded_jit_config,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            error!(
                "Failed to create the JIT runner {}: {e:?}",
                runner_name.hostname
            );
            // treat the runner as busy, but not finished yet
            group.status.send_modify(|status| {
                status.idle -= 1;
            });
            // try to delete the runner from github
            if let Err(e) = client
                .actions()
                .delete_org_runner(&*config.app.org_name, resp.runner.id)
                .await
            {
                error!(
                    "Failed to delete failed JIT runner {}: {e:?}",
                    runner_name.hostname
                );
            }
            // wait like 15s so that we don't end up spamming the github API
            artificial_delay(start_time).await;
            // mark it as done
            group.status.send_modify(|status| {
                status.total -= 1;
            });
            return;
        }
    };

    info!("Runner {} started", runner_name.hostname);

    // wait for the runner to signal to us that it has started a job
    {
        let mut buf = [0u8; 256];
        let mut signal_subscriber = config.shutdown_requested.subscribe();
        // we kinda don't care about what the runner sends us
        // all that matters is that it sent *something*
        let mut should_kill = false;
        select! {
            piperes = runner.ready_signal_pipe.read(&mut buf) => {
                if let Err(e) = piperes {
                    // assume an error to read from the signal pipe means the worker has started a job
                    // ... or that it crashed (the result is the same)
                    warn!(
                        "Failed to read from signal pipe for runner {}: {e:?}",
                        runner_name.hostname
                    );
                }
            },
            // alternatively, we might get SIGINT/TERM'd instead
            sigres = signal_subscriber.wait_for(Clone::clone) => {
                if let Err(e) = sigres {
                    warn!(
                        "Got error while waiting for SIGINT/TERM in runner {}: {e:?}",
                        runner_name.hostname
                    );
                }
                should_kill = true;
            }
        };
        // drop le pipe (closing it)
        drop(runner.ready_signal_pipe);

        // if we got sigterm'd, kill the runner
        if should_kill && let Err(e) = kill_runner(&runner_name.hostname).await {
            warn!("Failed to kill runner {}: {e:?}", runner_name.hostname);
        }
        // if we did not get sigterm'd, let this runner keep running in the background should we get
        // sigterm'd later
        if !should_kill {
            guard.exclude();
        }
    }
    info!("Runner {} picked up a job", runner_name.hostname);
    group.status.send_modify(|status| {
        status.idle -= 1;
    });

    // wait for the runner to quit
    if let Err(e) = runner.process.wait().await {
        // again - we kinda don't care about the error, we can assume the runner finished anyway
        warn!(
            "Failed to wait for runner {} to finish: {e:?}",
            runner_name.hostname
        );
    }
    // prevent the server from quitting until we're done with cleanup
    guard.include();
    info!("Runner {} finished a job", runner_name.hostname);

    // runner may have been terminated externally, check if it's been removed from github
    if client
        .actions()
        .get_org_runner(&*config.app.org_name, resp.runner.id)
        .await
        .is_ok()
    {
        info!(
            "Runner {} still exists in github after quitting - deleting",
            runner_name.hostname
        );
        // delete
        if let Err(e) = client
            .actions()
            .delete_org_runner(&*config.app.org_name, resp.runner.id)
            .await
        {
            error!(
                "Failed to delete failed JIT runner {}: {e:?}",
                runner_name.hostname
            );
        }
    }

    artificial_delay(start_time).await;
    group.status.send_modify(|status| {
        status.total -= 1;
    });
}

/// manages a group of runners, spawning new runners as necessary
async fn manage_runner_group(
    client: &'static Octocrab,
    config: &'static config::Config,
    group: &'static config::GroupConfig,
) {
    let mut receiver = group.status.subscribe();
    let mut quit_receiver = config.shutdown_requested.subscribe();
    loop {
        select! {
            // wait for the amount of runners to be below target
            // we should never get Err from this, as we're actively holding the sender via group.status
            res = receiver
                .wait_for(|status| status.idle < group.max_idle && status.total < group.max_runners)
            => res.unwrap(),

            // alternatively, we might get SIGTERM'd - quit immediately
            _ = quit_receiver.wait_for(Clone::clone) => break,
        };

        // allocate a new runner
        info!("Group {} - allocating a new runner", group.group_id);
        group.status.send_modify(|status| {
            status.idle += 1;
            status.total += 1;
        });
        tokio::spawn(spawn_runner(
            client,
            config,
            group,
            config.idle_counter.clone().into_guard(),
        ));
    }
}

async fn log_runner_group_updates(
    group_id: RunnerGroupId,
    mut receiver: tokio::sync::watch::Receiver<GroupStatus>,
) {
    loop {
        {
            let status = receiver.borrow_and_update();
            info!(
                "Group {group_id} status: {} total, {} idle",
                status.total, status.idle
            );
            drop(status);
        }
        if receiver.changed().await.is_err() {
            break;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), ErrorContext> {
    // load config & setup
    tracing_subscriber::fmt::init();
    let config = Box::leak(
        config::Config::get()
            .context("Failed to load config file")?
            .into(),
    );
    let client = Box::leak(
        config
            .app
            .build_client()
            .context("Failed to build client from config file data")?
            .into(),
    );
    std::fs::create_dir_all(&config.program.log_dir)
        .context("Failed to create the log directory")?;

    // spawn the signal watch task
    {
        let sender = config.shutdown_requested.clone();
        let mut signals =
            Signals::new([Signal::Int, Signal::Term]).context("Failed to listen for signals")?;
        tokio::spawn(async move {
            signals.next().await;
            sender.send_replace(true);
        });
    }

    // spawn management tasks for each group
    for group in &*config.groups {
        tokio::spawn(log_runner_group_updates(
            group.group_id,
            group.status.subscribe(),
        ));
        tokio::spawn(manage_runner_group(client, config, group));
    }

    // wait for sigint/sigterm
    config
        .shutdown_requested
        .subscribe()
        .wait_for(Clone::clone)
        .await
        .context("Error while waiting for exit signal")?;
    info!("SIGINT/TERM received, waiting for idle runners to quit");
    config
        .idle_counter
        .wait()
        .await
        .context("Error while waiting for runners")?;
    Ok(())
}
