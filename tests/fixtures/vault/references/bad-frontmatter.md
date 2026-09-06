---
title: "Bad Frontmatter Test"
related: [[Some Page]], [[Other Page]]
---

# Bad frontmatter test page

This file mirrors the cagan-shape bug: unquoted [[wikilinks]] inside a YAML
flow value break the parser (`did not find expected key`). It has no
`summa:` key, so — once parsing is fixed to survive it — it is reported as
malformed frontmatter but never treated as a summa page.
