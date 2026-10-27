# Crawler Polyglot Project

A production-ready documentation crawling and hosting service featuring a high-performance Rust worker and a Java (Quarkus) management service.

## Architecture

- **DocTech (Java/Quarkus)**: Provides the web UI, REST API, and manages crawler execution. Uses SLF4J for logging.
- **Spider (Rust)**: A high-performance, multithreaded crawler optimized for documentation. Supports `robots.txt` compliance, IP blacklisting, and CSS/HTML asset rewriting for offline use.

---

## Prerequisites

- **Java**: JDK 21+ (JDK 25 recommended for development)
- **Rust**: Cargo 1.80+
- **Maven**: 3.9+
- **Podman/Docker**: For container image builds

---

## Build Instructions

The project uses a unified Maven build system that handles both the Java and Rust components.

### Full Project Build
```bash
mvn clean install
```
*This will:*
1. Compile the Rust `spider` worker in release mode.
2. Place the `crawler` binary in `doctech/src/main/jib/usr/local/bin`.
3. Build and package the Java `doctech` service.

### Sub-module Development
- **Rust Only**: `cargo build --release --manifest-path spider/Cargo.toml`
- **Java Only**: `mvn clean compile -pl doctech`

---

## Configuration

### Environment Variables
Production-ready configuration should be provided via environment variables or a `.env` file.

| Variable | Description | Default |
| :--- | :--- | :--- |
| `DocTechHome` | Root path where crawled documentation is stored | `/tmp` |
| `RUST_LOG` | Logging level for the Rust crawler | `info` |

---

## Running the Application

### Development Mode (Quarkus)
Ensure the `crawler` binary is in your system `PATH`.
```bash
export DocTechHome=$(pwd)/storage
mvn quarkus:dev -pl doctech
```

### Production Execution
The application is accessible at `http://localhost:8080/docs`.

---

## Logging

Standardized logging is implemented across both components:
- **Java**: Logs are written to `STDOUT` and `${java.io.tmpdir}/doctech.log` (with 10MB rotation).
- **Rust**: Sub-process logs are captured by Java and prefixed with `[CRAWLER-OUT]` and `[CRAWLER-ERR]`.

---

## Security Features

- **Path Traversal Protection**: Both Java and Rust components validate paths and project names.
- **Protocol Enforcement**: Only `http` and `https` schemes are allowed.
- **SSRF Hardening**: The Rust crawler includes a built-in IP blacklist for internal ranges (127.0.0.1, 192.168.x.x, etc.).

---

## Deployment (Containerization)

Build a container image using the pre-configured Jib profile:
```bash
mvn clean package -Pjib -Dquarkus.container-image.build=true
```


## Quick Start (No Auth)
```bash
./install-build.sh
export DocTechHome=/var/lib/crawler/docs
java -DDocTechHome=$DocTechHome -jar doctech/target/quarkus-app/quarkus-run.jar
```
App: http://localhost:8080/docs
