package org.acme.doctech.web;

import jakarta.annotation.security.RolesAllowed;
import jakarta.inject.Inject;
import jakarta.ws.rs.Consumes;
import jakarta.ws.rs.POST;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.core.Response;
import org.acme.doctech.model.CrawlRequest;
import org.acme.doctech.service.CrawlService;
import org.acme.doctech.service.DocCache;

@Path("/api")
@RolesAllowed("teacher")
public class APIResource {
  @Inject DocCache cache;
  @Inject CrawlService crawlService;

  @POST
  @Path("refresh")
  public Response refresh() {
    cache.refresh();
    return Response.ok("Cache refreshed").build();
  }

  @POST
  @Path("crawl")
  @Consumes(MediaType.APPLICATION_JSON)
  public Response crawl(CrawlRequest request) {
    try {
      crawlService.startCrawl(request);
      return Response.accepted("Crawl started for " + request.name()).build();
    } catch (IllegalArgumentException e) {
      return Response.status(Response.Status.BAD_REQUEST).entity(e.getMessage()).build();
    }
  }
}
