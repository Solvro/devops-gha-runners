# actions runner images

list of images:

- `base`: the base image - just the runner on debian:stable and nothing else
- `node`: `base` + a bunch of node versions installed via nvm, using 26 by default, with pnpm preinstalled
- `website-deploy`: `node` + awscli, ssh, sshpass, rsync
