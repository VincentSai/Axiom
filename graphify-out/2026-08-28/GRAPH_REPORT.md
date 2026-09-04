# Graph Report - Axiom  (2026-08-28)

## Corpus Check
- 93 files · ~57,115 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1595 nodes · 3227 edges · 107 communities (106 shown, 1 thin omitted)
- Extraction: 92% EXTRACTED · 8% INFERRED · 0% AMBIGUOUS · INFERRED: 260 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `a666d86b`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- parse_module
- tier4_world.py
- CliError
- TranspileError
- Module
- e2e_pipeline_test.rs
- main.rs
- AgentTaskPool
- HandlerRegistry
- tier0_spatial.ts
- SPEC_200.md
- diagnose_source
- WitError
- models.py
- tier4_world.rs
- AstDatabase
- tier0_spatial.py
- tier2_combat.py
- tier1_stats.py
- tier4_world.ts
- axiom-vfs/src/lib.rs
- VfsError
- docs/ROADMAP.md
- tier3_ai_loot.rs
- NodeHash
- ecosystem_e2e_test.rs
- tier3_ai_loot.py
- pack_coords_to_hash
- tier1_stats.rs
- docs/MULTI_AGENT_WORKFLOW.md
- calc_manhattan_dist
- tier2_combat.rs
- Axiom Language Syntax & Semantics Specification (Draft v0.1)
- Axiom 语言语法与语义规范草案 (Language Specification v0.1)
- Axiom 语言语法与语义规范草案 (Language Specification v0.1)
- Axiom 語言語法與語意規範草案 (Language Specification v0.1)
- tier0_spatial.rs
- TestDirectory
- package.json
- index.ts
- README.md
- README.en.md
- compilerOptions
- 2. 核心架构决策矩阵 (Core Decisions)
- 2. Core Architectural Decisions
- 2. 核心架构决策矩阵 (Core Decisions)
- 2. 核心架構決策矩陣 (Core Decisions)
- calc_chebyshev_dist
- calc_grid_index
- ast_db_test.rs
- rs/src/lib.rs
- README.zh-CN.md
- README.zh-TW.md
- wrap_map_coord_y
- Axiom SMT 反例自愈指南 (Counter-Example Auto-Healing Guide)
- Axiom 多 Agent 并发开发协议与内容寻址模型
- Axiom 多 Agent 並行開發協定與內容定址模型
- 📐 三、5 大子系统 200 个真实业务函数规格
- Axiom 项目发展路线图 (Roadmap)
- bresenham_step_x
- bresenham_step_y
- calc_euclidean_sq
- calc_line_of_sight_cost
- clamp_to_bounds_y
- spatial_subsys_018
- spatial_subsys_021
- spatial_subsys_023
- spatial_subsys_024
- spatial_subsys_025
- spatial_subsys_026
- spatial_subsys_027
- spatial_subsys_028
- spatial_subsys_029
- spatial_subsys_030
- spatial_subsys_031
- spatial_subsys_032
- Q: Implement Ticket 17 CLI integration for export-wit, export-ffi, and scaffold, with ecosystem end-to-end tests.
- Axiom 架构师 Agent 指南 (Axiom Architecture & Spec Guide)
- Axiom Worker 极简填洞指南 (Axiom Pure Expression Guide)
- stat_subsys_055
- stat_subsys_056
- stat_subsys_053
- stat_subsys_066
- stat_subsys_064
- stat_subsys_054
- stat_subsys_063
- stat_subsys_067
- stat_subsys_070
- stat_subsys_071
- stat_subsys_072
- stat_subsys_073
- stat_subsys_074
- stat_subsys_075
- stat_subsys_076
- stat_subsys_077
- stat_subsys_079
- stat_subsys_080
- stat_subsys_081
- stat_subsys_082
- stat_subsys_083
- unpack_hash_x
- unpack_hash_y
- wrap_map_coord_y

## God Nodes (most connected - your core abstractions)
1. `parse_module()` - 53 edges
2. `ParseError` - 49 edges
3. `Expr` - 46 edges
4. `Parser<'a>` - 39 edges
5. `Module` - 30 edges
6. `CliError` - 29 edges
7. `FfiError` - 27 edges
8. `NodeHash` - 26 edges
9. `FunctionDecl` - 23 edges
10. `parse_expr()` - 22 edges

## Surprising Connections (you probably didn't know these)
- `assembles_a_hole_free_module_that_transpiles_to_rust()` --calls--> `transpile_to_rust()`  [INFERRED]
  crates/axiom-agent-pool/tests/concurrent_pool_test.rs → crates/axiom-transpiler/src/lib.rs
- `verifies_logical_operators_and_let_bindings()` --calls--> `parse_module()`  [INFERRED]
  crates/axiom-smt/tests/smt_solver_test.rs → crates/axiom-parser/src/lib.rs
- `world_engine_192()` --calls--> `calc_max_hp_from_vit()`  [INFERRED]
  examples/benchmark_200/rs/src/tier4_world.rs → examples/benchmark_200/rs/src/tier1_stats.rs
- `world_engine_195()` --calls--> `calc_weighted_loot_tier()`  [INFERRED]
  examples/benchmark_200/rs/src/tier4_world.rs → examples/benchmark_200/rs/src/tier3_ai_loot.rs
- `unit_variant()` --references--> `VariantDef`  [EXTRACTED]
  crates/axiom-wit/src/lib.rs → crates/axiom-ast/src/lib.rs

## Import Cycles
- None detected.

## Communities (107 total, 1 thin omitted)

### Community 0 - "parse_module"
Cohesion: 0.09
Nodes (46): Box, Expr, accepts_calls_when_the_caller_declares_the_callee_effects(), MockHandler, rejects_a_call_when_the_caller_does_not_propagate_an_effect(), rejects_effect_names_without_a_declaration(), transfer_example_checks_and_resolves_mock_handlers(), parse_expr() (+38 more)

### Community 1 - "tier4_world.py"
Cohesion: 0.07
Nodes (53): calc_manhattan_dist(), Return the Manhattan distance between two grid coordinates., Wrap ``x`` into the half-open interval ``[0, width)``., wrap_map_coord_x(), calc_armor_damage_reduction(), calc_lifesteal_conversion(), calc_max_hp_from_vit(), calc_phys_atk_from_str() (+45 more)

### Community 2 - "CliError"
Cohesion: 0.11
Nodes (48): check(), Cli, CliError, Commands, compile_and_run(), database_export(), database_put(), DatabaseCommands (+40 more)

### Community 3 - "TranspileError"
Cohesion: 0.14
Nodes (25): BinaryOperator, binary_operator(), find_record_type_for_expr(), is_scalar_type(), Display, Error, Formatter, From (+17 more)

### Community 5 - "Module"
Cohesion: 0.09
Nodes (56): EffectDecl, EffectOp, FieldDef, FunctionDecl, FunctionParam, Literal, MatchArm, Module (+48 more)

### Community 6 - "e2e_pipeline_test.rs"
Cohesion: 0.29
Nodes (14): assert_success(), axiom(), check_extract_holes_and_emit_rust_report_invalid_inputs(), cli_happy_paths_cover_check_stdout_json_emit_and_run(), db_put_and_export_round_trip_a_content_addressed_module(), extracts_transfer_implementation_as_schema_compliant_json(), filled_axiom_source_emits_rust_that_compiles_and_runs(), path_text() (+6 more)

### Community 9 - "AgentTaskPool"
Cohesion: 0.09
Nodes (33): CounterExample, AgentTaskPool, PoolError, replace_hole(), Display, Error, Formatter, From (+25 more)

### Community 10 - "HandlerRegistry"
Cohesion: 0.07
Nodes (28): assembles_a_hole_free_module_that_transpiles_to_rust(), assert_success(), Drop, Output, Path, PathBuf, Self, String (+20 more)

### Community 11 - "tier0_spatial.ts"
Cohesion: 0.10
Nodes (44): bresenham_step_x(), bresenham_step_y(), calc_chebyshev_dist(), calc_euclidean_sq(), calc_grid_index(), calc_line_of_sight_cost(), calc_manhattan_dist(), check_aabb_overlap() (+36 more)

### Community 12 - "SPEC_200.md"
Cohesion: 0.07
Nodes (35): 📊 一、全自动打洞流水线实测数据矩阵 (/axiom-pipeline), 🔬 三、技术原理解析：为什么打洞流水线具备巨大优势？, 🥊 二、两大语言多 Agent 协作实测权威大比拼 (Token Benchmark Comparison), 📊 一、5 个 Python Codex Agent 并发协作实测数据矩阵, 🔬 三、跨语言多 Agent 协作实测横向对比, ⚡ 二、单元测试性能, 📊 一、5 个 Rust Codex Agent 并发协作实测数据矩阵, 🔬 三、Rust 与 Axiom 多 Agent 协作维度对比 (+27 more)

### Community 14 - "diagnose_source"
Cohesion: 0.09
Nodes (34): diagnose_module(), diagnose_source(), DiagnosticReport, DiagnosticStatus, effect_report(), format_expr(), HashMap, Option (+26 more)

### Community 15 - "WitError"
Cohesion: 0.14
Nodes (28): generate_wit(), generate_wit_from_source(), normalize_package_name(), render_custom_type(), render_effect(), render_function_export(), render_function_type(), render_type() (+20 more)

### Community 16 - "models.py"
Cohesion: 0.09
Nodes (18): BaseAttributes, DerivedStats, Entity, Point, Domain data models for the roguelike world simulation engine., A two-dimensional velocity vector., An inclusive axis-aligned rectangle., An entity's unmodified attributes. (+10 more)

### Community 17 - "tier4_world.rs"
Cohesion: 0.17
Nodes (27): calc_grid_index(), clamp_to_bounds_x(), pack_coords_to_hash(), wrap_map_coord_x(), calc_crit_chance_clamped(), calc_phys_atk_from_str(), scale_drop_by_luck(), determine_turn_initiative() (+19 more)

### Community 18 - "AstDatabase"
Cohesion: 0.17
Nodes (12): AstDatabase, DbError, Error, Formatter, From, Option, Path, PathBuf (+4 more)

### Community 19 - "tier0_spatial.py"
Cohesion: 0.08
Nodes (23): check_aabb_overlap(), check_in_radius_sq(), Tier 0 spatial geometry and grid-math primitives. The functions in this module…, Return whether two valid axis-aligned boxes overlap inclusively., Return quadtree child ``b`` within parent-cell range ``a``., Return the shared length of two origin-aligned intervals., Return the start offset for range ``a`` with stride ``b + 1``., Return whether a nonnegative squared distance is within ``radius``. (+15 more)

### Community 20 - "tier2_combat.py"
Cohesion: 0.08
Nodes (18): Return a Cartesian quadrant code: NE=0, NW=1, SW=2, SE=3., Return the product of two nonnegative projection scales., Return the grid bucket containing coordinate ``a`` at resolution ``b``., Return nonnegative cell-local coordinate ``a`` modulo ``b``., spatial_subsys_017(), spatial_subsys_022(), spatial_subsys_034(), spatial_subsys_035() (+10 more)

### Community 21 - "tier1_stats.py"
Cohesion: 0.09
Nodes (18): Return 1 when ``a`` lies below half-plane boundary ``b``, else 0., Return row width in a radius-``a`` diamond at vertical offset ``b``., Return collision penetration bounded by both nonnegative depths., Return a Bresenham-style doubled raster-error update., spatial_subsys_036(), spatial_subsys_037(), spatial_subsys_039(), spatial_subsys_040() (+10 more)

### Community 22 - "tier4_world.ts"
Cohesion: 0.21
Nodes (22): determine_turn_initiative(), requireIntegerInRange(), resolve_boss_phase_transition(), resolve_terrain_hazard_tick(), simulate_single_turn_exchange(), validateWorldClock(), world_engine_185(), world_engine_186() (+14 more)

### Community 23 - "axiom-vfs/src/lib.rs"
Cohesion: 0.20
Nodes (17): UnaryOperator, escape_string(), export_database_module(), export_module_to_source(), expr_precedence(), format_expr(), format_inline_block(), format_literal() (+9 more)

### Community 24 - "VfsError"
Cohesion: 0.13
Nodes (14): ModuleRecord, Vec, Display, Error, Formatter, From, Option, Result (+6 more)

### Community 25 - "docs/ROADMAP.md"
Cohesion: 0.12
Nodes (15): Axiom Project Roadmap, Phase 1: MVP Prototype & Core Compiler (Q1), Phase 2: Effect System & Formal Contract Verification (Q2), Phase 3: Content-Addressed AST Codebase & Multi-Agent Concurrency (Q3), Phase 4: Cross-Platform Runtimes & Ecosystem Interop (Q4), Axiom 项目发展路线图 (Roadmap), 阶段一：原型与验证编译器 (MVP Prototype - Q1), 阶段三：内容寻址 AST 代码库与多 Agent 协同 (Q3) (+7 more)

### Community 26 - "tier3_ai_loot.rs"
Cohesion: 0.11
Nodes (15): stat_subsys_057(), stat_subsys_060(), stat_subsys_062(), stat_subsys_065(), combat_subsys_107(), combat_subsys_110(), combat_subsys_112(), combat_subsys_115() (+7 more)

### Community 27 - "NodeHash"
Cohesion: 0.22
Nodes (13): AsRef, compute_effect_hash(), compute_function_hash(), compute_hash(), compute_module_hash(), compute_type_hash(), NodeHash, ParseNodeHashError (+5 more)

### Community 28 - "ecosystem_e2e_test.rs"
Cohesion: 0.28
Nodes (12): assert_success(), axiom(), export_ffi_selects_typescript_and_c_outputs(), export_wit_writes_a_component_interface(), path_text(), Drop, Output, Path (+4 more)

### Community 29 - "tier3_ai_loot.py"
Cohesion: 0.12
Nodes (13): Public API for the pure five-tier roguelike engine benchmark., clamp_to_bounds_x(), Clamp ``x`` into the half-open map interval ``[0, width)``., calc_crit_chance_clamped(), stat_subsys_060(), stat_subsys_062(), combat_subsys_110(), combat_subsys_112() (+5 more)

### Community 30 - "pack_coords_to_hash"
Cohesion: 0.12
Nodes (17): pack_coords_to_hash(), Pack nonnegative ``x`` and the low 16 bits of ``y`` into an integer., Extract the high-coordinate X component from a packed hash., Extract the low 16-bit Y component from a packed hash., unpack_hash_x(), unpack_hash_y(), stat_subsys_057(), stat_subsys_058() (+9 more)

### Community 31 - "tier1_stats.rs"
Cohesion: 0.12
Nodes (12): spatial_subsys_035(), spatial_subsys_036(), spatial_subsys_038(), calc_effective_penetration(), calc_max_hp_from_vit(), stat_subsys_085(), stat_subsys_086(), stat_subsys_088() (+4 more)

### Community 32 - "docs/MULTI_AGENT_WORKFLOW.md"
Cohesion: 0.15
Nodes (12): 1. Why Traditional Git Fails Multi-Agent Concurrency, 2. Axiom's Content-Addressed AST Solution, 3. Standard Operating Procedure (SOP) for Multi-Agent Workflows, 4. Automated Healing via SMT Counterexamples, Axiom Multi-Agent Collaboration Protocol & Content-Addressed Codebase, Core Capabilities:, 1. 为什么传统 Git 不适合多 Agent 并行？, 2. Axiom 的内容寻址 AST 解决方案 (+4 more)

### Community 33 - "calc_manhattan_dist"
Cohesion: 0.13
Nodes (16): calc_manhattan_dist(), spatial_subsys_018(), spatial_subsys_028(), calc_lifesteal_conversion(), stat_subsys_051(), stat_subsys_068(), stat_subsys_078(), apply_bleed_stack_tick() (+8 more)

### Community 34 - "tier2_combat.rs"
Cohesion: 0.13
Nodes (10): spatial_subsys_034(), spatial_subsys_040(), calc_armor_damage_reduction(), stat_subsys_084(), stat_subsys_090(), apply_poison_exponential_decay(), combat_subsys_134(), combat_subsys_140() (+2 more)

### Community 35 - "Axiom Language Syntax & Semantics Specification (Draft v0.1)"
Cohesion: 0.13
Nodes (15): 1. Lexical Conventions & Syntax Basics, 2.1 Primitive Types, 2.2 Algebraic Data Types (ADTs), 2. Type System, 3.1 Defining an Effect, 3.2 Declaring Effect Dependencies on Functions, 3. Algebraic Effect System, 4. Design by Contract (+7 more)

### Community 36 - "Axiom 语言语法与语义规范草案 (Language Specification v0.1)"
Cohesion: 0.13
Nodes (15): 1. 基础语法与词法规则, 2.1 原生类型 (Primitive Types), 2.2 代数数据类型 (ADT), 2. 类型系统 (Type System), 3.1 定义 Effect, 3.2 声明函数依赖的 Effect, 3. 代数效应系统 (Algebraic Effect System), 4. 契约驱动设计 (Design by Contract) (+7 more)

### Community 37 - "Axiom 语言语法与语义规范草案 (Language Specification v0.1)"
Cohesion: 0.13
Nodes (15): 1. 基础语法与词法规则, 2.1 原生类型 (Primitive Types), 2.2 代数数据类型 (ADT), 2. 类型系统 (Type System), 3.1 定义 Effect, 3.2 声明函数依赖的 Effect, 3. 代数效应系统 (Algebraic Effect System), 4. 契约驱动设计 (Design by Contract) (+7 more)

### Community 38 - "Axiom 語言語法與語意規範草案 (Language Specification v0.1)"
Cohesion: 0.13
Nodes (15): 1. 基礎語法與詞法規則, 2.1 原生型別 (Primitive Types), 2.2 代數資料型別 (ADT), 2. 型別系統 (Type System), 3.1 定義 Effect, 3.2 宣告函數依賴的 Effect, 3. 代數效應系統 (Algebraic Effect System), 4. 契約驅動設計 (Design by Contract) (+7 more)

### Community 39 - "tier0_spatial.rs"
Cohesion: 0.14
Nodes (14): calc_chebyshev_dist(), spatial_subsys_019(), spatial_subsys_037(), spatial_subsys_039(), stat_subsys_052(), stat_subsys_069(), stat_subsys_087(), stat_subsys_089() (+6 more)

### Community 40 - "TestDirectory"
Cohesion: 0.24
Nodes (9): axiom(), faulty_contract_emits_machine_readable_counter_example(), path_text(), Drop, Output, Path, PathBuf, Self (+1 more)

### Community 41 - "package.json"
Cohesion: 0.15
Nodes (12): devDependencies, tsx, @types/node, typescript, name, scripts, test, type (+4 more)

### Community 42 - "index.ts"
Cohesion: 0.33
Nodes (9): BaseAttributes, DerivedStats, Entity, Point, Rect, StatusEffect, Velocity, BinaryEngineFunction (+1 more)

### Community 43 - "README.md"
Cohesion: 0.21
Nodes (7): 💻 CLI 工具链完整指南, 🔬 为什么 Axiom 打洞流水线具备颠覆性优势？, 📊 对比热门编程语言, 🏆 权威基准评测：200-Function 真实业务多 Agent 协作大比拼, 🌟 核心愿景与问题背景, 🛠️ 语法速览 (Syntax Glance), 📂 项目文档导航

### Community 44 - "README.en.md"
Cohesion: 0.21
Nodes (7): 💻 CLI Toolchain Guide, 📊 Comparison with Mainstream Languages, 🌟 Core Vision & Problem Statement, 📂 Documentation Navigator, 🏆 Industrial Benchmark: 200-Function Multi-Agent Concurrency, 🛠️ Syntax Glance, 🔬 Why Does Axiom's Hole Pipeline Outperform Traditional Languages?

### Community 45 - "compilerOptions"
Cohesion: 0.17
Nodes (11): compilerOptions, esModuleInterop, forceConsistentCasingInFileNames, module, moduleResolution, skipLibCheck, strict, target (+3 more)

### Community 46 - "2. 核心架构决策矩阵 (Core Decisions)"
Cohesion: 0.20
Nodes (10): 1. 架构设计哲学 (Architectural Philosophy), 2. 核心架构决策矩阵 (Core Decisions), 3. 系统数据流图 (End-to-End Pipeline), Axiom 架构设计白皮书 (Architecture Whitepaper), 决策 1：编译底层与跨平台策略 —— 【A+C 混合】, 决策 2：计算范式与状态模型 —— 【纯函数 + 代数效应 + 契约】, 决策 3：代码组织模型 —— 【内容寻址 AST (Content-Addressed)】, 决策 4：表层语法设计 —— 【现代极简表达式】 (+2 more)

### Community 47 - "2. Core Architectural Decisions"
Cohesion: 0.20
Nodes (10): 1. Architectural Philosophy, 2. Core Architectural Decisions, 3. End-to-End Pipeline, Axiom Architecture Whitepaper, Decision 1: Compilation Target & Cross-Platform Strategy — [Hybrid Transpilation], Decision 2: Computation Paradigm & State Model — [Pure Functions + Algebraic Effects + Contracts], Decision 3: Codebase Organization — [Content-Addressed AST], Decision 4: Surface Syntax Design — [Modern Minimalist Expressions] (+2 more)

### Community 48 - "2. 核心架构决策矩阵 (Core Decisions)"
Cohesion: 0.20
Nodes (10): 1. 架构设计哲学 (Architectural Philosophy), 2. 核心架构决策矩阵 (Core Decisions), 3. 系统数据流图 (End-to-End Pipeline), Axiom 架构设计白皮书 (Architecture Whitepaper), 决策 1：编译底层与跨平台策略 —— 【A+C 混合】, 决策 2：计算范式与状态模型 —— 【纯函数 + 代数效应 + 契约】, 决策 3：代码组织模型 —— 【内容寻址 AST (Content-Addressed)】, 决策 4：表层语法设计 —— 【现代极简表达式】 (+2 more)

### Community 49 - "2. 核心架構決策矩陣 (Core Decisions)"
Cohesion: 0.20
Nodes (10): 1. 架構設計哲學 (Architectural Philosophy), 2. 核心架構決策矩陣 (Core Decisions), 3. 系統資料流圖 (End-to-End Pipeline), Axiom 架構設計白皮書 (Architecture Whitepaper), 決策 1：編譯底層與跨平臺策略 —— 【A+C 混合】, 決策 2：計算範式與狀態模型 —— 【純函數 + 代數效應 + 契約】, 決策 3：代碼組織模型 —— 【內容定址 AST (Content-Addressed)】, 決策 4：表層語法設計 —— 【現代極簡運算式】 (+2 more)

### Community 50 - "calc_chebyshev_dist"
Cohesion: 0.20
Nodes (10): calc_chebyshev_dist(), Return the farthest nonnegative ray-intersection depth., Return the Chebyshev (king-move) distance between two points., spatial_subsys_019(), stat_subsys_052(), stat_subsys_069(), combat_subsys_102(), combat_subsys_119() (+2 more)

### Community 51 - "calc_grid_index"
Cohesion: 0.22
Nodes (9): calc_grid_index(), Flatten a nonnegative grid coordinate into a row-major index., stat_subsys_065(), combat_subsys_115(), ai_loot_subsys_162(), Return P1 above 66%, P2 at 33%-66%, and P3 below 33%., Count completed world cycles and map them into checkpoint space., resolve_boss_phase_transition() (+1 more)

### Community 52 - "ast_db_test.rs"
Cohesion: 0.46
Nodes (7): hashes_are_stable_and_change_when_content_changes(), inserts_a_module_and_round_trips_every_ast_node(), persisted_database_can_reconstruct_the_original_module(), PathBuf, sample_module(), temporary_database_path(), textual_hashes_round_trip_and_reject_malformed_values()

### Community 53 - "rs/src/lib.rs"
Cohesion: 0.39
Nodes (7): BaseAttributes, DerivedStats, Entity, Point, Rect, StatusEffect, Velocity

### Community 54 - "README.zh-CN.md"
Cohesion: 0.25
Nodes (7): 💻 CLI 工具链完整指南, 🔬 为什么 Axiom 打洞流水线具备颠覆性优势？, 📊 对比热门编程语言, 🏆 权威基准评测：200-Function 真实业务多 Agent 协作大比拼, 🌟 核心愿景与问题背景, 🛠️ 语法速览 (Syntax Glance), 📂 项目文档导航

### Community 55 - "README.zh-TW.md"
Cohesion: 0.25
Nodes (7): 💻 CLI 工具鏈完整指南, 📂 專案文件導覽, 📊 對比熱門程式語言, 🌟 核心願景與問題背景, 🏆 權威基準評測：200-Function 真實業務多 Agent 協作大比拼, 🔬 為什麼 Axiom 打洞管線具備顛覆性優勢？, 🛠️ 語法速覽 (Syntax Glance)

### Community 56 - "wrap_map_coord_y"
Cohesion: 0.29
Nodes (7): Wrap ``y`` into the half-open interval ``[0, height)``., wrap_map_coord_y(), stat_subsys_061(), combat_subsys_111(), ai_loot_subsys_158(), Return turns remaining until the next cycle boundary., world_engine_189()

### Community 58 - "Axiom SMT 反例自愈指南 (Counter-Example Auto-Healing Guide)"
Cohesion: 0.29
Nodes (6): 1. 结构化诊断报告解读 (DiagnosticReport Schema), 2. 四步自愈思维链 (4-Step CoT SOP), 3. 典型反例与修复范例, Axiom SMT 反例自愈指南 (Counter-Example Auto-Healing Guide), 范例 1：透支与下溢 (Underflow / Overdraft), 范例 2：除以零 (Division by Zero)

### Community 59 - "Axiom 多 Agent 并发开发协议与内容寻址模型"
Cohesion: 0.33
Nodes (6): 1. 为什么传统 Git 不适合多 Agent 并行？, 2. Axiom 的内容寻址 AST 解决方案, 3. 多 Agent 并发协作标准作业流程 (SOP), 4. 自动反例自愈协议 (Auto-Healing with Counter-Examples), Axiom 多 Agent 并发开发协议与内容寻址模型, 核心特性：

### Community 60 - "Axiom 多 Agent 並行開發協定與內容定址模型"
Cohesion: 0.33
Nodes (6): 1. 為什麼傳統 Git 不適合多 Agent 並行？, 2. Axiom 的內容定址 AST 解決方案, 3. 多 Agent 並發協作標準作業程序 (SOP), 4. 自動反例自癒協定 (Auto-Healing with Counter-Examples), Axiom 多 Agent 並行開發協定與內容定址模型, 核心特性：

### Community 61 - "📐 三、5 大子系统 200 个真实业务函数规格"
Cohesion: 0.33
Nodes (6): 📐 三、5 大子系统 200 个真实业务函数规格, 🔹 模块 1：空间几何与网格系统 (40 函数，`grid_math_001` ~ `grid_math_040`), 🔹 模块 2：属性映射与抗性计算系统 (50 函数，`stat_calc_041` ~ `stat_calc_090`), 🔹 模块 3：混合战斗与 Buff 状态机系统 (50 函数，`combat_flow_091` ~ `combat_flow_140`), 🔹 模块 4：怪物 AI 决策与加权掉落系统 (40 函数，`ai_loot_141` ~ `ai_loot_180`), 🔹 模块 5：回合流转与世界仿真引擎 (20 函数，`world_engine_181` ~ `world_engine_200`)

### Community 62 - "Axiom 项目发展路线图 (Roadmap)"
Cohesion: 0.40
Nodes (5): Axiom 项目发展路线图 (Roadmap), 阶段一：原型与验证编译器 (MVP Prototype - Q1), 阶段三：内容寻址 AST 代码库与多 Agent 协同 (Q3), 阶段二：Effect 系统与契约验证 (Q2), 阶段四：全平台与生态互操作 (Q4)

### Community 63 - "bresenham_step_x"
Cohesion: 0.40
Nodes (5): bresenham_step_x(), Return the horizontal sign step from ``x1`` toward ``x2``., stat_subsys_055(), combat_subsys_105(), ai_loot_subsys_152()

### Community 64 - "bresenham_step_y"
Cohesion: 0.40
Nodes (5): bresenham_step_y(), Return the vertical sign step from ``y1`` toward ``y2``., stat_subsys_056(), combat_subsys_106(), ai_loot_subsys_153()

### Community 65 - "calc_euclidean_sq"
Cohesion: 0.40
Nodes (5): calc_euclidean_sq(), Return squared Euclidean distance without computing a square root., stat_subsys_053(), combat_subsys_103(), ai_loot_subsys_150()

### Community 66 - "calc_line_of_sight_cost"
Cohesion: 0.40
Nodes (5): calc_line_of_sight_cost(), Return terrain-weighted line-of-sight energy cost, rounded down., stat_subsys_066(), combat_subsys_116(), ai_loot_subsys_163()

### Community 67 - "clamp_to_bounds_y"
Cohesion: 0.40
Nodes (5): clamp_to_bounds_y(), Clamp ``y`` into the half-open map interval ``[0, height)``., stat_subsys_063(), combat_subsys_113(), ai_loot_subsys_160()

### Community 68 - "spatial_subsys_018"
Cohesion: 0.40
Nodes (5): Return the diamond-field mask distance from the origin., spatial_subsys_018(), stat_subsys_068(), combat_subsys_118(), ai_loot_subsys_165()

### Community 69 - "spatial_subsys_021"
Cohesion: 0.40
Nodes (5): Return the squared displacement of grid vector ``(a, b)``., spatial_subsys_021(), stat_subsys_071(), combat_subsys_121(), ai_loot_subsys_168()

### Community 70 - "spatial_subsys_023"
Cohesion: 0.40
Nodes (5): Return the smaller of two nonnegative axis extents., spatial_subsys_023(), stat_subsys_073(), combat_subsys_123(), ai_loot_subsys_170()

### Community 71 - "spatial_subsys_024"
Cohesion: 0.40
Nodes (5): Return the sum of the normalized directions of two axes., spatial_subsys_024(), stat_subsys_074(), combat_subsys_124(), ai_loot_subsys_171()

### Community 72 - "spatial_subsys_025"
Cohesion: 0.40
Nodes (5): Return the absolute span delta between two axis positions., spatial_subsys_025(), stat_subsys_075(), combat_subsys_125(), ai_loot_subsys_172()

### Community 73 - "spatial_subsys_026"
Cohesion: 0.40
Nodes (5): Return the floor midpoint of a nonnegative interval., spatial_subsys_026(), stat_subsys_076(), combat_subsys_126(), ai_loot_subsys_173()

### Community 74 - "spatial_subsys_027"
Cohesion: 0.40
Nodes (5): Pack parent cell ``a`` and bounded local coordinate ``b``., spatial_subsys_027(), stat_subsys_077(), combat_subsys_127(), ai_loot_subsys_174()

### Community 75 - "spatial_subsys_028"
Cohesion: 0.40
Nodes (5): Return the Manhattan ring number for offset ``(a, b)``., spatial_subsys_028(), stat_subsys_078(), combat_subsys_128(), ai_loot_subsys_175()

### Community 76 - "spatial_subsys_029"
Cohesion: 0.40
Nodes (5): Return the area of a nonnegative grid rectangle., spatial_subsys_029(), stat_subsys_079(), combat_subsys_129(), ai_loot_subsys_176()

### Community 77 - "spatial_subsys_030"
Cohesion: 0.40
Nodes (5): Return the squared diagonal length of a grid cell., spatial_subsys_030(), stat_subsys_080(), combat_subsys_130(), ai_loot_subsys_177()

### Community 78 - "spatial_subsys_031"
Cohesion: 0.40
Nodes (5): Advance a nonnegative ray origin ``a`` by step ``b``., spatial_subsys_031(), stat_subsys_081(), combat_subsys_131(), ai_loot_subsys_178()

### Community 79 - "spatial_subsys_032"
Cohesion: 0.40
Nodes (5): Return remaining visibility after nonnegative occlusion ``b``., spatial_subsys_032(), stat_subsys_082(), combat_subsys_132(), ai_loot_subsys_179()

### Community 80 - "Q: Implement Ticket 17 CLI integration for export-wit, export-ffi, and scaffold, with ecosystem end-to-end tests."
Cohesion: 0.40
Nodes (4): Answer, Outcome, Q: Implement Ticket 17 CLI integration for export-wit, export-ffi, and scaffold, with ecosystem end-to-end tests., Source Nodes

### Community 81 - "Axiom 架构师 Agent 指南 (Axiom Architecture & Spec Guide)"
Cohesion: 0.40
Nodes (4): 1. 核心心智模型 (Diff Mapping 迁移指南), 2. 规范骨架代码范例 (Golden Pattern), 3. 架构师防错禁令 (Anti-Patterns), Axiom 架构师 Agent 指南 (Axiom Architecture & Spec Guide)

### Community 82 - "Axiom Worker 极简填洞指南 (Axiom Pure Expression Guide)"
Cohesion: 0.40
Nodes (4): 1. 核心语法卡片 (Expression CheatSheet), 2. 预训练常见幻觉反例表 (Hallucination Defense), 3. 黄金准则, Axiom Worker 极简填洞指南 (Axiom Pure Expression Guide)

### Community 83 - "stat_subsys_055"
Cohesion: 0.50
Nodes (4): bresenham_step_x(), stat_subsys_055(), combat_subsys_105(), ai_loot_subsys_152()

### Community 84 - "stat_subsys_056"
Cohesion: 0.50
Nodes (4): bresenham_step_y(), stat_subsys_056(), combat_subsys_106(), ai_loot_subsys_153()

### Community 85 - "stat_subsys_053"
Cohesion: 0.50
Nodes (4): calc_euclidean_sq(), stat_subsys_053(), combat_subsys_103(), ai_loot_subsys_150()

### Community 86 - "stat_subsys_066"
Cohesion: 0.50
Nodes (4): calc_line_of_sight_cost(), stat_subsys_066(), combat_subsys_116(), ai_loot_subsys_163()

### Community 87 - "stat_subsys_064"
Cohesion: 0.50
Nodes (4): check_aabb_overlap(), stat_subsys_064(), combat_subsys_114(), ai_loot_subsys_161()

### Community 88 - "stat_subsys_054"
Cohesion: 0.50
Nodes (4): check_in_radius_sq(), stat_subsys_054(), combat_subsys_104(), ai_loot_subsys_151()

### Community 89 - "stat_subsys_063"
Cohesion: 0.50
Nodes (4): clamp_to_bounds_y(), stat_subsys_063(), combat_subsys_113(), ai_loot_subsys_160()

### Community 90 - "stat_subsys_067"
Cohesion: 0.50
Nodes (4): spatial_subsys_017(), stat_subsys_067(), combat_subsys_117(), ai_loot_subsys_164()

### Community 91 - "stat_subsys_070"
Cohesion: 0.50
Nodes (4): spatial_subsys_020(), stat_subsys_070(), combat_subsys_120(), ai_loot_subsys_167()

### Community 92 - "stat_subsys_071"
Cohesion: 0.50
Nodes (4): spatial_subsys_021(), stat_subsys_071(), combat_subsys_121(), ai_loot_subsys_168()

### Community 93 - "stat_subsys_072"
Cohesion: 0.50
Nodes (4): spatial_subsys_022(), stat_subsys_072(), combat_subsys_122(), ai_loot_subsys_169()

### Community 94 - "stat_subsys_073"
Cohesion: 0.50
Nodes (4): spatial_subsys_023(), stat_subsys_073(), combat_subsys_123(), ai_loot_subsys_170()

### Community 95 - "stat_subsys_074"
Cohesion: 0.50
Nodes (4): spatial_subsys_024(), stat_subsys_074(), combat_subsys_124(), ai_loot_subsys_171()

### Community 96 - "stat_subsys_075"
Cohesion: 0.50
Nodes (4): spatial_subsys_025(), stat_subsys_075(), combat_subsys_125(), ai_loot_subsys_172()

### Community 97 - "stat_subsys_076"
Cohesion: 0.50
Nodes (4): spatial_subsys_026(), stat_subsys_076(), combat_subsys_126(), ai_loot_subsys_173()

### Community 98 - "stat_subsys_077"
Cohesion: 0.50
Nodes (4): spatial_subsys_027(), stat_subsys_077(), combat_subsys_127(), ai_loot_subsys_174()

### Community 99 - "stat_subsys_079"
Cohesion: 0.50
Nodes (4): spatial_subsys_029(), stat_subsys_079(), combat_subsys_129(), ai_loot_subsys_176()

### Community 100 - "stat_subsys_080"
Cohesion: 0.50
Nodes (4): spatial_subsys_030(), stat_subsys_080(), combat_subsys_130(), ai_loot_subsys_177()

### Community 101 - "stat_subsys_081"
Cohesion: 0.50
Nodes (4): spatial_subsys_031(), stat_subsys_081(), combat_subsys_131(), ai_loot_subsys_178()

### Community 102 - "stat_subsys_082"
Cohesion: 0.50
Nodes (4): spatial_subsys_032(), stat_subsys_082(), combat_subsys_132(), ai_loot_subsys_179()

### Community 103 - "stat_subsys_083"
Cohesion: 0.50
Nodes (4): spatial_subsys_033(), stat_subsys_083(), combat_subsys_133(), ai_loot_subsys_180()

### Community 104 - "unpack_hash_x"
Cohesion: 0.50
Nodes (4): unpack_hash_x(), stat_subsys_058(), combat_subsys_108(), ai_loot_subsys_155()

### Community 105 - "unpack_hash_y"
Cohesion: 0.50
Nodes (4): unpack_hash_y(), stat_subsys_059(), combat_subsys_109(), ai_loot_subsys_156()

### Community 106 - "wrap_map_coord_y"
Cohesion: 0.50
Nodes (4): wrap_map_coord_y(), stat_subsys_061(), combat_subsys_111(), ai_loot_subsys_158()

## Knowledge Gaps
- **198 isolated node(s):** `Parser`, `Velocity`, `Rect`, `name`, `version` (+193 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Module` connect `Module` to `parse_module`, `CliError`, `TranspileError`, `AgentTaskPool`, `diagnose_source`, `WitError`, `AstDatabase`, `ast_db_test.rs`, `axiom-vfs/src/lib.rs`, `NodeHash`?**
  _High betweenness centrality (0.026) - this node is a cross-community bridge._
- **Why does `parse_module()` connect `parse_module` to `CliError`, `TranspileError`, `Module`, `AgentTaskPool`, `diagnose_source`, `WitError`, `axiom-vfs/src/lib.rs`, `VfsError`?**
  _High betweenness centrality (0.026) - this node is a cross-community bridge._
- **Why does `FunctionDecl` connect `Module` to `parse_module`, `CliError`, `TranspileError`, `AgentTaskPool`, `HandlerRegistry`, `diagnose_source`, `WitError`, `AstDatabase`, `NodeHash`?**
  _High betweenness centrality (0.019) - this node is a cross-community bridge._
- **Are the 41 inferred relationships involving `parse_module()` (e.g. with `.from_source()` and `check()`) actually correct?**
  _`parse_module()` has 41 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Parser`, `Velocity`, `Rect` to the rest of the system?**
  _198 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `parse_module` be split into smaller, more focused modules?**
  _Cohesion score 0.08716540837336993 - nodes in this community are weakly interconnected._
- **Should `tier4_world.py` be split into smaller, more focused modules?**
  _Cohesion score 0.07197763801537387 - nodes in this community are weakly interconnected._