#!/bin/bash
set -xeuo pipefail

# temporarily reenable caching (since we do a cache mount)
mv /etc/apt/apt.conf.d/docker-{clean,gzip-indexes} /tmp/

# packages
apt-get update
apt-get upgrade -y
apt-get install -y awscli openssh-client sshpass rsync

# disable caching again
mv /tmp/docker-{clean,gzip-indexes} /etc/apt/apt.conf.d/
