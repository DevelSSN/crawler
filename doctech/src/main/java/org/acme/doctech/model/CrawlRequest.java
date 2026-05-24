package org.acme.doctech.model;

public record CrawlRequest(
    String name, String url, int depth, int workers, boolean hardcodeExternal) {}
