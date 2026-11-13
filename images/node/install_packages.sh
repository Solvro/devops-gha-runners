#!/bin/bash
set -xeuo pipefail

# temporarily reenable caching (since we do a cache mount)
mv /etc/apt/apt.conf.d/docker-{clean,gzip-indexes} /tmp/

# packages
apt-get update
apt-get upgrade -y

# install everything libnode needs, except node packages
apt-get -y install `apt-cache depends libnode115 | grep -v "node" | awk '/Depends:/{print$2}'`

# extra deps that node needs
apt-get install -y libatomic1

# disable caching again
mv /tmp/docker-{clean,gzip-indexes} /etc/apt/apt.conf.d/
