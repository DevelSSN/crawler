#!/usr/bin/env sh
set -eu

DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export DocTechHome="${DocTechHome:-$DIR/docs}"
mkdir -p "$DocTechHome"
export PATH="$DIR:$PATH"

exec java ${JAVA_OPTS:-} -jar "$DIR/quarkus-run.jar" "$@"
