#!/bin/bash
set -xeuo pipefail
# packages
apt-get update
apt-get upgrade -y
apt-get install -y curl jq liblttng-ust1t64 libkrb5-3 zlib1g libssl3t64 libicu76
apt-get clean

# fetch release
curl -o /tmp/release.json "https://api.github.com/repos/actions/runner/releases/tags/$RUNNER_VERSION"
jq '.assets | map(select(.name | startswith("actions-runner-linux-x64-")))[0] | {hash: .digest | ltrimstr("sha256:"), url: .browser_download_url}' /tmp/release.json > /tmp/asset.json
RUNNER_HASH="`jq -r '.hash' /tmp/asset.json`"
RUNNER_URL="`jq -r '.url' /tmp/asset.json`"

curl -Lo /tmp/runner.tar.gz "$RUNNER_URL"
echo "$RUNNER_HASH  /tmp/runner.tar.gz" | sha256sum -c

# extract
mkdir -p /opt/gha
cd /opt/gha
tar -xf /tmp/runner.tar.gz
cp /ctx/entrypoint.sh /entrypoint.sh

# create user
groupadd -g 1000 runner
useradd -u 1000 -g 1000 -s /bin/bash -d /runner runner
mkdir /runner
chown runner:runner /runner
chown -R runner:runner /opt/gha
