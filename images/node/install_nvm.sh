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
sed -ie 's/curl -q/curl/g' /tmp/nvm-install.sh
PROFILE=/runner/.load_nvm /tmp/nvm-install.sh
echo ". /runner/.load_nvm" >> /runner/.bashprofile
