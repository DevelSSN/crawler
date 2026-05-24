package org.acme.doctech.service;

import module java.base;

import jakarta.enterprise.context.ApplicationScoped;
import org.acme.doctech.model.DocProject;
import org.acme.doctech.model.DocScanner;
import org.eclipse.microprofile.config.inject.ConfigProperty;

@ApplicationScoped
public class LocalDocScanner implements DocScanner {
  @ConfigProperty(name = "docs.root.path")
  String rootPath;

  @Override
  public List<DocProject> scanProjects() {
    Path root = Paths.get(rootPath);
    try (var stream = Files.list(root)) {
      return stream
          .filter(Files::isDirectory)
          .filter(p -> !p.getFileName().toString().startsWith("."))
          .map(this::mapToProject)
          .filter(Objects::nonNull)
          .sorted(Comparator.comparing(DocProject::name))
          .toList();
    } catch (IOException e) {
      return List.of();
    }
  }

  private DocProject mapToProject(Path projectDir) {
    try (var walk = Files.walk(projectDir, 5)) {
      Path indexFile =
          walk.filter(p -> p.getFileName().toString().equalsIgnoreCase("index.html"))
              .min(Comparator.comparingInt(Path::getNameCount))
              .orElse(null);
      if (indexFile == null) return null;
      String relativePath = projectDir.relativize(indexFile).toString();
      LocalDateTime mtime =
          LocalDateTime.ofInstant(
              Files.getLastModifiedTime(projectDir).toInstant(), ZoneId.systemDefault());
      return new DocProject(projectDir.getFileName().toString(), relativePath, mtime);
    } catch (IOException e) {
      return null;
    }
  }
}
