package org.acme.doctech.web;

import io.vertx.ext.web.Router;
import io.vertx.ext.web.handler.FileSystemAccess;
import io.vertx.ext.web.handler.StaticHandler;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.enterprise.event.Observes;
import org.eclipse.microprofile.config.inject.ConfigProperty;

@ApplicationScoped
public class DocRouter {
  @ConfigProperty(name = "docs.root.path")
  String rootPath;

  /**
   * This method is called during Quarkus startup. It mounts the external documentation folder to
   * the Vert.x Router.
   */
  void init(@Observes Router router) {
    // We mount to "/v1/*" because the app's base path is already /crawl-docs
    router
        .route("/v1/*")
        .handler(StaticHandler.create(FileSystemAccess.ROOT, rootPath).setDirectoryListing(false));
  }
}
