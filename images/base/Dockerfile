FROM docker.io/library/debian:stable
ARG RUNNER_VERSION

# install packages separately from runner so that we can unmount the apt cache for the runner step
# just in case someone decides to run malware in the runner install step that could poison the cache
RUN \
  --mount=type=bind,dst=/ctx \
  --mount=type=tmpfs,dst=/tmp \
  --mount=type=cache,id=debian-cache,dst=/var/cache \
  --mount=type=cache,id=debian-apt,dst=/var/lib/apt \
  /ctx/install_packages.sh
RUN \
  --mount=type=bind,dst=/ctx \
  --mount=type=tmpfs,dst=/tmp \
  RUNNER_VERSION="$RUNNER_VERSION" /ctx/install_runner.sh

WORKDIR /runner
USER runner:runner
ENV ACTIONS_RUNNER_HOOK_JOB_STARTED=/pre_job.sh
ENTRYPOINT ["/entrypoint.sh"]
