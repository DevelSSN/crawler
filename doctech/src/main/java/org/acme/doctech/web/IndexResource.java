package org.acme.doctech.web;

import io.quarkus.qute.Template;
import io.quarkus.qute.TemplateInstance;
import jakarta.annotation.security.RolesAllowed;
import jakarta.inject.Inject;
import jakarta.ws.rs.Consumes;
import jakarta.ws.rs.DefaultValue;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import org.acme.doctech.model.CrawlRequest;
import org.acme.doctech.service.CrawlService;
import org.acme.doctech.service.DocCache;
import org.jboss.resteasy.reactive.RestForm;

@Path("/ui")
public class IndexResource {
  @Inject DocCache cache;
  @Inject CrawlService crawlService;
  @Inject Template index;
  @Inject Template crawl;

  @GET
  @Produces(MediaType.TEXT_HTML)
  public TemplateInstance get() {
    return index.data("projects", cache.getProjects());
  }

  @GET
  @Path("crawl")
  @Produces(MediaType.TEXT_HTML)
  @RolesAllowed("teacher")
  public TemplateInstance crawlForm() {
    return crawl.instance();
  }

  @POST
  @Path("crawl")
  @Consumes(MediaType.APPLICATION_FORM_URLENCODED)
  @RolesAllowed("teacher")
  public Response processCrawl(
      @RestForm String name,
      @RestForm String url,
      @RestForm @DefaultValue("3") int depth,
      @RestForm @DefaultValue("4") int workers,
      @RestForm boolean hardcodeExternal) {

    crawlService.startCrawl(new CrawlRequest(name, url, depth, workers, hardcodeExternal));
    return Response.seeOther(java.net.URI.create("/ui")).build();
  }
}
