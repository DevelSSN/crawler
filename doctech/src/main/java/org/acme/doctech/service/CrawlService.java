package org.acme.doctech.service;

import module java.base;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.acme.doctech.model.CrawlRequest;
import org.eclipse.microprofile.config.inject.ConfigProperty;
import org.eclipse.microprofile.context.ManagedExecutor;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

@ApplicationScoped
public class CrawlService {
  private static final Logger log = LoggerFactory.getLogger(CrawlService.class);

  @ConfigProperty(name = "docs.root.path")
  String rootPath;

  @Inject ManagedExecutor executor;
  @Inject DocCache cache;

  private void validateRequest(CrawlRequest request) {
    if (request.name() == null || !request.name().matches("^[a-zA-Z0-9_-]+$")) {
      throw new IllegalArgumentException(
          "Invalid project name: must be alphanumeric (dashes/underscores allowed)");
    }
    if (request.depth() < 0 || request.depth() > 5) {
      throw new IllegalArgumentException("Invalid depth: must be between 0 and 5");
    }
    if (request.workers() < 1 || request.workers() > 10) {
      throw new IllegalArgumentException("Invalid workers: must be between 1 and 10");
    }
    if (request.url() == null
        || (!request.url().startsWith("http://") && !request.url().startsWith("https://"))) {
      throw new IllegalArgumentException("Invalid URL: only http and https protocols are allowed");
    }
  }

  public void startCrawl(CrawlRequest request) {
    validateRequest(request);
    executor.runAsync(
        () -> {
          try {
            log.info("Starting crawl for project: {} at URL: {}", request.name(), request.url());

            // Log version for audit
            Process versionProcess = new ProcessBuilder("crawler", "--version").start();
            logStream(versionProcess.getInputStream(), "CRAWLER-VER");
            versionProcess.waitFor();

            var outputPath = Paths.get(rootPath).resolve(request.name()).toAbsolutePath();

            ProcessBuilder pb =
                new ProcessBuilder(
                    "crawler",
                    "--index",
                    request.url(),
                    "--output",
                    outputPath.toString(),
                    "--depth",
                    String.valueOf(request.depth()),
                    "--workers",
                    String.valueOf(request.workers()));

            if (request.hardcodeExternal()) {
              pb.command().add("--hardcode-external");
            }

            Process process = pb.start();
            
            // Capture stdout and stderr
            executor.execute(() -> logStream(process.getInputStream(), "CRAWLER-OUT"));
            executor.execute(() -> logStream(process.getErrorStream(), "CRAWLER-ERR"));

            int exitCode = process.waitFor();

            if (exitCode == 0) {
              log.info("Crawl completed successfully for project: {}", request.name());
              cache.refresh();
            } else {
              log.error("Crawler failed for project: {} with exit code: {}", request.name(), exitCode);
            }
          } catch (Exception e) {
            log.error("Unexpected error during crawl for project: {}", request.name(), e);
          }
        });
  }

  private void logStream(InputStream is, String prefix) {
    try (BufferedReader reader = new BufferedReader(new InputStreamReader(is))) {
      String line;
      while ((line = reader.readLine()) != null) {
        log.info("[{}] {}", prefix, line);
      }
    } catch (IOException e) {
      log.warn("Error reading subprocess stream: {}", prefix, e);
    }
  }
}
