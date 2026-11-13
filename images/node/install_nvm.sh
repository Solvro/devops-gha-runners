#!/bin/bash
set -xeuo pipefail

# get latest nvm release
curl --fail -o /tmp/release.json "https://api.github.com/repos/nvm-sh/nvm/releases/latest"
LATEST_NVM_VERSION="`jq -r '.tag_name' /tmp/release.json`"

# download the install script
curl --fail -Lo /tmp/nvm-install.sh "https://raw.githubusercontent.com/nvm-sh/nvm/$LATEST_NVM_VERSION/install.sh"
chmod +x /tmp/nvm-install.sh

# install nvm
touch /runner/.load_nvm
PROFILE=/runner/.load_nvm /tmp/nvm-install.sh
echo ". /runner/.load_nvm" >> /runner/.bashprofile

# preinstall a bunch of node runtimes
. /runner/.load_nvm
nvm install 22
nvm install 24
nvm install 26
nvm alias default 26

# update all the package manager versions, install pnpm in all versions
for VERSION in 22 24 26
do
  nvm use $VERSION
  npm i -g --allow-scripts=pnpm npm pnpm
done
nvm use default
