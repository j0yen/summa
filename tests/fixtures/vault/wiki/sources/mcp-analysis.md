---
summa: source-summary
source: Clippings/mcp-analysis.pdf
ingested: 2026-06-21T01:00:00Z
title: MCP Server Codebase Analysis
entities: ["[[Model Context Protocol]]", "[[AtScale]]"]
---

TL;DR: This document analyzes the MCP server codebase for AtScale's semantic layer, covering tool dispatch and column metadata exposure.

Key claims:
- The MCP server exposes semantic models as queryable tools.
- Column groups are cached per model version.
- Tool schemas are registered and available for batch loading.

Links to: [[Model Context Protocol]]
