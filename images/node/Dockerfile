FROM ghcr.io/solvro/devops-gha-runners:base

# install packages separately from runner so that we can unmount the apt cache for the runner step
# just in case someone decides to run malware in the runner install step that could poison the cache
RUN \
  --mount=type=bind,dst=/ctx \
  --mount=type=tmpfs,dst=/tmp \
  --mount=type=cache,id=debian-cache,dst=/var/cache \
  --mount=type=cache,id=debian-apt,dst=/var/lib/apt \
  sudo /ctx/install_packages.sh
RUN \
  --mount=type=bind,dst=/ctx \
  --mount=type=tmpfs,dst=/tmp \
  --mount=type=cache,id=nvm-cache,dst=/runner/.nvm/.cache \
  --mount=type=cache,id=npm-cache,dst=/runner/.npm \
  /ctx/install_nvm.sh

# get nvm to always run
ENV BASH_ENV="/runner/.load_nvm"
