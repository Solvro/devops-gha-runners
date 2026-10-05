FROM docker.io/library/debian:stable
ARG RUNNER_VERSION
RUN --mount=type=bind,dst=/ctx --mount=type=tmpfs,dst=/tmp /ctx/install.sh
WORKDIR /runner
USER runner:runner
ENV ACTIONS_RUNNER_HOOK_JOB_STARTED=/pre_job.sh
ENTRYPOINT ["/entrypoint.sh"]
