#!/bin/bash
set -euo pipefail

# read jitconfig via fd 3
JITCONFIG="`cat /dev/fd/3`"
exec 3<&-

# run github actions runner
/opt/gha/run.sh --jitconfig "$JITCONFIG"
