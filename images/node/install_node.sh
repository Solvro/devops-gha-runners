#!/bin/bash
set -euo pipefail

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
