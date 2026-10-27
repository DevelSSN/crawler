# Crawler Polyglot Project

A production-ready documentation crawling and hosting service featuring a high-performance Rust worker and a Java (Quarkus) management service. Authentication is intentionally not included: the service is meant to run behind your own network boundary or reverse proxy.

## Architecture

- **DocTech (Java/Quarkus)**: Web UI, REST API, static documentation hosting, and crawler process management.
- **Spider (Rust)**: Multithreaded crawler with `robots.txt` compliance, IP blacklisting, and offline asset rewriting.
- **Runtime relationship**: DocTech spawns the `crawler` executable from `PATH`. Every deployment must provide both DocTech and the Rust binary.

---

## Prerequisites

- **Java**: JDK 25 (`maven.compiler.release` is `25`)
- **Maven**: 3.9+
- **Rust**: 1.85+ (`edition = "2024"`)
- **Podman or Docker**: only for container image builds
- **GraalVM 25**: only for native builds

---

## Repository Layout

| Path | Purpose |
| :--- | :--- |
| `doctech/` | Quarkus application |
| `spider/` | Rust crawler |
| `scripts/run-jvm.sh` | Launcher for JVM bundles |
| `scripts/run-native.sh` | Launcher for native bundles |
| `.github/workflows/release.yml` | Tag-triggered release workflow |
| `install-build.sh` | Convenience build script |
| `dist/` | Local production artifacts (gitignored) |

---

## Build

### Full JVM build

```bash
mvn clean package -DskipTests
```

This:
1. Builds the Rust worker in release mode (`spider/target/release/crawler`).
2. Copies the crawler into `doctech/src/main/jib/usr/local/bin`.
3. Builds the Quarkus fast-jar at `doctech/target/quarkus-app/`.

Use `./install-build.sh` for the same result, or `./install-build.sh --no-skip-tests` to run tests.

### Uber-jar build

```bash
mvn clean package -DskipTests -Dquarkus.package.jar.type=uber-jar
```

Output: `doctech/target/doctech-1.0.0-SNAPSHOT-runner.jar`.

### Native build

```bash
mvn clean package -Pnative -DskipTests -Dquarkus.native.native-image-xmx=4g
```

Output: `doctech/target/doctech-1.0.0-SNAPSHOT-runner`.

Native builds are not part of the release workflow yet.

### Sub-module builds

```bash
cargo build --release --manifest-path spider/Cargo.toml
mvn clean compile -pl doctech
```

---

## Production Artifacts (Local)

The following reproduces exactly what the GitHub release workflow publishes. `TAG` is the current tag, e.g. `v1.1`.

### 1. Build and bundle the fast-jar

```bash
TAG=$(git describe --tags --abbrev=0)
mvn -B clean package -DskipTests
NAME="crawler-${TAG}-jvm-fastjar"
mkdir -p "dist/${NAME}/docs"
cp -r doctech/target/quarkus-app/. "dist/${NAME}/"
cp spider/target/release/crawler "dist/${NAME}/"
cp scripts/run-jvm.sh "dist/${NAME}/run.sh"
chmod +x "dist/${NAME}/run.sh" "dist/${NAME}/crawler"
(cd dist && zip -qr "${NAME}.zip" "${NAME}")
```

### 2. Build and bundle the uber-jar

```bash
mvn -B clean package -DskipTests -Dquarkus.package.jar.type=uber-jar
NAME="crawler-${TAG}-jvm-uber"
JAR=$(find doctech/target -maxdepth 1 -name '*-runner.jar' -type f | head -n1)
mkdir -p "dist/${NAME}/docs"
cp "$JAR" "dist/${NAME}/quarkus-run.jar"
cp spider/target/release/crawler "dist/${NAME}/"
cp scripts/run-jvm.sh "dist/${NAME}/run.sh"
chmod +x "dist/${NAME}/run.sh" "dist/${NAME}/crawler"
(cd dist && zip -qr "${NAME}.zip" "${NAME}")
cp "$JAR" "dist/crawler-${TAG}-uber.jar"
cp spider/target/release/crawler "dist/crawler-${TAG}-linux-x86_64"
chmod +x "dist/crawler-${TAG}-linux-x86_64"
cp install-build.sh dist/
```

### 3. Generate checksums

```bash
(cd dist && find . -maxdepth 1 -type f ! -name 'SHA256SUMS.txt' -printf '%f\n' \
  | sort | xargs sha256sum > SHA256SUMS.txt)
```

### 4. Verify

```bash
(cd dist && sha256sum -c SHA256SUMS.txt)
./dist/crawler-v1.1-linux-x86_64 --version
```

### Output

| Artifact | Contents |
| :--- | :--- |
| `crawler-<tag>-jvm-fastjar.zip` | Quarkus fast-jar, `crawler`, `run.sh`, `docs/` |
| `crawler-<tag>-jvm-uber.zip` | Uber jar as `quarkus-run.jar`, `crawler`, `run.sh`, `docs/` |
| `crawler-<tag>-uber.jar` | Raw uber jar |
| `crawler-<tag>-linux-x86_64` | Raw Rust crawler binary |
| `install-build.sh` | Build helper |
| `SHA256SUMS.txt` | SHA-256 checksums |

---

## Running in Production

### Fast-jar bundle

```bash
unzip crawler-v1.1-jvm-fastjar.zip
cd crawler-v1.1-jvm-fastjar
DocTechHome=/var/lib/crawler/docs ./run.sh
```

### Uber-jar bundle

```bash
unzip crawler-v1.1-jvm-uber.zip
cd crawler-v1.1-jvm-uber
DocTechHome=/var/lib/crawler/docs ./run.sh
```

### Raw uber jar

Install the Rust binary as `crawler` in `PATH`, then:

```bash
export DocTechHome=/var/lib/crawler/docs
java -jar crawler-v1.1-uber.jar
```

### Application defaults

- URL: `http://<host>:8080/docs`
- Cache refresh: `POST /docs/api/refresh`
- Crawl: `POST /docs/api/crawl` (JSON)
- File log: `${java.io.tmpdir}/doctech.log`
- Docs root: `${DocTechHome}` (default `/tmp`)

### systemd example

```ini
[Unit]
Description=Crawler DocTech
After=network.target

[Service]
User=crawler
Group=crawler
WorkingDirectory=/opt/crawler
Environment=DocTechHome=/var/lib/crawler/docs
Environment=JAVA_OPTS=-Xms256m -Xmx1g
ExecStart=/opt/crawler/run.sh
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Deploy:

```bash
sudo useradd --system --home /opt/crawler --shell /usr/sbin/nologin crawler
sudo mkdir -p /opt /var/lib/crawler/docs
sudo unzip crawler-<tag>-jvm-fastjar.zip -d /opt
sudo mv /opt/crawler-<tag>-jvm-fastjar /opt/crawler
sudo chown -R crawler:crawler /opt/crawler /var/lib/crawler
sudo systemctl daemon-reload
sudo systemctl enable --now crawler
sudo systemctl status crawler
```

### Container image

```bash
mvn clean package -Pjib -Dquarkus.container-image.build=true
```

Jib is configured to use Podman. The image name defaults to the Maven coordinates (`org.acme.crawler/doctech:1.0.0-SNAPSHOT`). Override as needed:

```bash
mvn clean package -Pjib \
  -Dquarkus.container-image.build=true \
  -Dquarkus.container-image.group=myorg \
  -Dquarkus.container-image.name=crawler \
  -Dquarkus.container-image.tag=v1.1
```

Push to a registry:

```bash
mvn clean package -Pjib \
  -Dquarkus.container-image.build=true \
  -Dquarkus.container-image.push=true \
  -Dquarkus.container-image.registry=registry.example.com \
  -Dquarkus.container-image.group=myorg
```

Run:

```bash
podman run --rm -p 8080:8080 \
  -v /var/lib/crawler/docs:/data/docs:Z \
  -e DocTechHome=/data/docs \
  org.acme.crawler/doctech:1.0.0-SNAPSHOT
```

---

## GitHub Release Automation

The workflow is `.github/workflows/release.yml` and runs only for tags matching `v*`.

### What it does

1. Checkout, JDK 25, Rust stable, Maven/Rust caches.
2. Job `jvm-fastjar`: builds and uploads `crawler-<tag>-jvm-fastjar.zip`.
3. Job `jvm-uber`: builds uber-jar, uploads the bundle zip, raw jar, and raw Rust binary.
4. Job `release`: downloads artifacts, copies `install-build.sh`, writes `SHA256SUMS.txt`, and publishes the GitHub release with generated notes.
5. Native Linux asset is intentionally not included yet.

### Publish a release

The workflow must exist on the tagged commit, and the repository must be authenticated for push.

```bash
git tag -f -a v1.1 -m "Release v1.1"
git push origin main
git push -f origin v1.1
```

If SSH is not configured:

```bash
gh auth login
gh auth setup-git
```

Then push again. Pushing the tag triggers the workflow; no manual release creation is needed.

### GitHub source archives

GitHub always attaches its own `Source code (zip)` and `Source code (tar.gz)` archives to a release. They cannot be removed or replaced. Custom build outputs are added as additional assets. `.gitattributes export-ignore` can only change their contents, not remove the archives.

---

## Configuration

| Variable | Description | Default |
| :--- | :--- | :--- |
| `DocTechHome` | Root directory for crawled documentation | `/tmp` |
| `RUST_LOG` | Rust crawler log level | `info` |
| `JAVA_OPTS` | JVM options used by `run.sh` | empty |
| `QUARKUS_HTTP_PORT` | HTTP port (or use `JAVA_OPTS=-Dquarkus.http.port=...`) | `8080` |

`run.sh` prepends its own directory to `PATH` and creates `DocTechHome` when missing, so the bundled `crawler` binary is always found.

---

## Logging

- **Java**: STDOUT plus `${java.io.tmpdir}/doctech.log`, rotated at 10 MB with 5 backups.
- **Rust**: Subprocess output is captured by DocTech and logged with `[CRAWLER-OUT]` / `[CRAWLER-ERR]` prefixes.

## Security Features

- **Path traversal protection**: Both Java and Rust validate paths and project names.
- **Protocol enforcement**: Only `http` and `https` are allowed.
- **SSRF hardening**: The Rust crawler blocks internal ranges (127.0.0.1, 192.168.x.x, etc.).
- **No authentication**: Expose only on trusted networks or behind an authenticated reverse proxy.

---

## Quick Start

```bash
./install-build.sh
export DocTechHome=/var/lib/crawler/docs
java -DDocTechHome="$DocTechHome" -jar doctech/target/quarkus-app/quarkus-run.jar
```

App: http://localhost:8080/docs
