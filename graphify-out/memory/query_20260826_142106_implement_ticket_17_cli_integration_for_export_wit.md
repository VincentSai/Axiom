---
type: "query"
date: "2026-08-26T14:21:06.783313+00:00"
question: "Implement Ticket 17 CLI integration for export-wit, export-ffi, and scaffold, with ecosystem end-to-end tests."
contributor: "graphify"
outcome: "useful"
source_nodes: ["Cli", "Commands", "run()", "check()", "e2e_pipeline_test.rs"]
---

# Q: Implement Ticket 17 CLI integration for export-wit, export-ffi, and scaffold, with ecosystem end-to-end tests.

## Answer

Expanded from original query via graph vocab: [axiom, cli, rust, check]. The graph locates the public CLI seam in crates/axiom-cli/src/lib.rs: Cli and Commands feed run(), with check() as an existing command implementation. Existing subprocess coverage is in crates/axiom-cli/tests/e2e_pipeline_test.rs. No graph vocabulary or nodes were found for WIT, FFI, export, or scaffold, so these integrations appear absent from the current graph and need verification in source.

## Outcome

- Signal: useful

## Source Nodes

- Cli
- Commands
- run()
- check()
- e2e_pipeline_test.rs