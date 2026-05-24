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

  public void startCrawl(CrawlRequest request) {
    executor.runAsync(
        () -> {
          try {
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
