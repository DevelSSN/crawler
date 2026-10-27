#!/usr/bin/env bash
set -euo pipefail

# Build script for crawler (spider + doctech)
# Usage: ./install-build.sh [--native] [--skip-tests]

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
NATIVE=0
SKIP_TESTS="-DskipTests"

for arg in "$@"; do
  case "$arg" in
    --native) NATIVE=1 ;;
    --no-skip-tests) SKIP_TESTS="" ;;
    *) ;;
  esac
done

echo "[build] starting at $ROOT"

# Ensure deps
command -v mvn >/dev/null 2>&1 || { echo "mvn not found"; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "cargo not found"; exit 1; }
command -v java >/dev/null 2>&1 || { echo "java not found"; exit 1; }

if [ $NATIVE -eq 1 ]; then
  echo "[build] native mode"
  mvn -f "$ROOT/pom.xml" clean package -Pnative $SKIP_TESTS
else
  echo "[build] jvm mode"
  mvn -f "$ROOT/pom.xml" clean package $SKIP_TESTS
fi

JAR="$ROOT/doctech/target/quarkus-app/quarkus-run.jar"
if [ ! -f "$JAR" ]; then
  # try fast-jar/uber
  JAR="$(find "$ROOT/doctech/target" -name '*-runner.jar' -o -name 'quarkus-run.jar' | head -n1)"
fi

echo "[build] done"
echo "[run] java -DDocTechHome=/path/to/docs -jar $JAR"
echo "[env] export DocTechHome=/var/lib/crawler/docs"
