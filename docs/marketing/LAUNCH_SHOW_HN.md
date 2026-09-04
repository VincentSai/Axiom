# Hacker News: Show HN Post Draft

### Title:
`Show HN: Axiom – A deterministic game logic DSL with SMT-LIB2/Z3 verification & Unity zero-copy bridge`

### URL:
`https://github.com/axiom-lang/axiom`

### Post Body:

Hi HN,

We built **Axiom**, an experimental domain-specific language (DSL) and research prototype exploring **deterministic, contract-verified game logic and micro-hole AI collaboration.**

### Why build this?
Developing core logic for multiplayer action games, roguelites, or competitive games often suffers from:
1. **Implicit state & floating-point divergence**: Unchecked mutable state or platform differences causing subtle sync drift or corrupted replays.
2. **AI prompt context bloat**: Asking LLMs to navigate and rewrite monolithic files leads to hallucinations and wasted tokens.
3. **Boundary flaws**: Damage formulas dealing negative numbers, division by zero, or unverified edge conditions.

### How Axiom works:
- **Pure Functional Logic**: Game states and transitions are written as side-effect-free pure functions.
- **SMT-LIB2 & Microsoft Z3 Verification**: Functions define `requires` and `ensures` contracts. Axiom translates these into standard SMT-LIB2 queries for first-order theorem proving with Microsoft Z3 (or CVC5), with a standalone symbolic fallback engine.
- **Micro Hole-Driven Tasks**: Functions can contain typed holes (`?hole`). The compiler extracts standalone `hole-task.json` tasks so AI agents only need to generate single expressions in minimal isolated contexts.
- **Zero-Copy Double-Buffered Bridge (`axiom bridge`)**: Transpiles to Rust and builds a native C-ABI shared library with auto-generated Unity C# P/Invoke and fixed-capacity double-buffering.

### Playable Showcase:
We included a fully playable 2D Roguelite Survivor showcase in `examples/game_survivor/` running on Unity with custom procedural pixel art, continuous memory snapshots, and instant restart.

- GitHub: https://github.com/axiom-lang/axiom
- VS Code Extension: `editors/vscode-axiom`

We would love your feedback on deterministic game state modeling, SMT verification pipelines, and AI-native language design!
