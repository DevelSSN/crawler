package org.acme.doctech.model;

import module java.base;

public record DocProject(String name, String relativeEntryPoint, LocalDateTime lastModified) {}
