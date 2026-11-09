use std::{
    os::{fd::AsRawFd, unix::fs::chown},
    process::Stdio,
};

use chrono::Utc;
use cloneable_errors::{ErrorContext, ResContext};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    net::unix::pipe,
    process::{Child, Command},
};

use crate::config::{GroupConfig, ProgramConfig};

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

pub struct Runner {
    pub process: Child,
    pub ready_signal_pipe: pipe::Receiver,
}

#[allow(clippy::too_many_lines)]
pub async fn create_runner(
    program: &ProgramConfig,
    group: &GroupConfig,
    runner_name: &str,
    jit_token: &str,
) -> Result<Runner, ErrorContext> {
    // prepare the logging directories
    let time = Utc::now().format("%Y%m%d-%H%M%S");
    let container_logfile = program.log_dir.join(format!("{time}-{runner_name}.log"));
    let container_logdir = program.log_dir.join(format!("{time}-{runner_name}"));
    tokio::fs::create_dir_all(&container_logdir)
        .await
        .with_context(|| {
            format!(
                "Failed to create the log directory {}",
                container_logdir.display()
            )
        })?;
    chown(
        &container_logdir,
        Some(program.log_dir_owner_uid),
        Some(program.log_dir_owner_gid),
    )
    .with_context(|| {
        format!(
            "Failed to chown the log directory {} to {}:{}",
            container_logdir.display(),
            group.log_dir_uid,
            group.log_dir_gid
        )
    })?;
    let container_logfile_param = format!(
        "path={}",
        container_logfile
            .to_str()
            .context("container_logfile is not valid utf-8???")?
    );
    let container_logdir_param = format!(
        "{}:/opt/gha/_diag:idmap=uids=@{}-{}-1;gids=@{}-{}-1",
        container_logdir
            .to_str()
            .context("container_logdir is not valid utf-8???")?,
        program.log_dir_owner_uid,
        group.log_dir_uid,
        program.log_dir_owner_gid,
        group.log_dir_gid,
    );

    // Prepare file descriptors to pass into the child proc
    //  -> set up the jit config temporary file
    let jit_file = {
        // first open the "temp directory" with O_TMPFILE, which creates a new unlinked file
        let mut file = File::options()
            .write(true)
            .custom_flags(libc::O_TMPFILE)
            .open(&program.tmp_dir)
            .await
            .with_context(|| {
                format!(
                    "Failed to create a temporary file at {}",
                    program.tmp_dir.display()
                )
            })?;
        // write our token to the file
        file.write_all(jit_token.as_bytes())
            .await
            .context("Failed to write the jit token to the jit config file")?;
        // reopen the file as read-only
        File::open(format!("/proc/self/fd/{}", file.as_raw_fd()))
            .await
            .context("Failed to reopen the temporary jit config file as read-only")?
    };
    //  -> open a new pipe for the runner to signal to us when it takes a job
    let (tx, rx) = pipe::pipe().context("Failed to open an anonymous pipe")?;

    // get fds of the files we want to pass into the runner
    let jit_fd = jit_file.as_raw_fd();
    let pipe_fd = tx.as_raw_fd();

    // spawn a new process
    let mut command = Command::new("podman");
    command
        .arg("run")
        .args(group.additional_options.iter().map(|s| &**s))
        .args([
            // remove the container automatically
            "--rm",
            // set hostname
            "-h",
            runner_name,
            // set name
            "--name",
            runner_name,
            // map userids
            "--userns",
            "auto",
            // pass down fd 3 and 4
            "--preserve-fds",
            "2",
            // log to file, not journald
            "--log-driver",
            "k8s-file",
            "--log-opt",
            &container_logfile_param,
            // mount a log directory, with the proper idmaps
            "-v",
            &container_logdir_param,
            // use the image set by the group
            &*group.image,
        ])
        .stdout(Stdio::null());
    unsafe {
        command.pre_exec(move || {
            // dup the fds into place
            if libc::dup2(jit_fd, 3) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::dup2(pipe_fd, 4) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let process = command
        .spawn()
        .context("Failed to spawn the runner process")?;
    drop((jit_file, tx));

    Ok(Runner {
        process,
        ready_signal_pipe: rx,
    })
}
