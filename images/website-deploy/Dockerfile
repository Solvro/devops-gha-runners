FROM ghcr.io/solvro/devops-gha-runners:node

RUN \
  --mount=type=bind,dst=/ctx \
  --mount=type=tmpfs,dst=/tmp \
  --mount=type=cache,id=debian-cache,dst=/var/cache \
  --mount=type=cache,id=debian-apt,dst=/var/lib/apt \
  sudo /ctx/install_packages.sh
