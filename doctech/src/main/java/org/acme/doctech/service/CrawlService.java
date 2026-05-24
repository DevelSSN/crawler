package org.acme.doctech.service;

import module java.base;

import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.acme.doctech.model.CrawlRequest;
import org.eclipse.microprofile.config.inject.ConfigProperty;
import org.eclipse.microprofile.context.ManagedExecutor;

@ApplicationScoped
public class CrawlService {
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
            // Log version for audit
            new ProcessBuilder("crawler", "--version").inheritIO().start().waitFor();

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
            // Redirect output to the Quarkus process logs
            pb.inheritIO();

            Process process = pb.start();
            int exitCode = process.waitFor();

            if (exitCode == 0) {
              cache.refresh();
            }
          } catch (Exception e) {
            e.printStackTrace();
          }
        });
  }
}
