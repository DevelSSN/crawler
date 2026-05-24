package org.acme.doctech.service;

import module java.base;

import io.quarkus.runtime.Startup;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.acme.doctech.model.DocProject;
import org.acme.doctech.model.DocScanner;

@ApplicationScoped
public class DocCache {
  private final Map<String, DocProject> cache = new ConcurrentHashMap<>();

  @Inject DocScanner scanner;

  @Startup
  void init() {
    refresh();
  }

  public void refresh() {
    List<DocProject> projects = scanner.scanProjects();
    cache.clear();
    for (DocProject p : projects) {
      cache.put(p.name(), p);
    }
  }

  public List<DocProject> getProjects() {
    return cache.values().stream().sorted(Comparator.comparing(DocProject::name)).toList();
  }
}
