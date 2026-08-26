# 11 — AST 节点规范化哈希计算与嵌入式内容寻址存储引擎 (ASTDB)

**What to build:**
构建 `crates/axiom-db`，实现对 Axiom AST 各层级节点（TypeDef, FunctionDecl, EffectDecl, Module）的确定性规范化哈希计算（SHA-256），并提供嵌入式内容寻址存储引擎，支持按节点哈希读写、版本索引与依赖图遍历。

**Blocked by:**
None — can start immediately

**Status:**
ready-for-agent

**Seams:**
- `axiom_db::NodeHash`
- `axiom_db::AstDatabase`
- `AstDatabase::open(path: &Path) -> Result<Self, DbError>`
- `AstDatabase::insert_module(&mut self, module: &Module) -> Result<ModuleSummary, DbError>`
- `AstDatabase::get_function(&self, hash: &NodeHash) -> Result<Option<FunctionDecl>, DbError>`

**Acceptance criteria:**
- [ ] 相同 AST 语义的节点计算出严格一致的规范化 Hash（忽略无关空格和注释）
- [ ] ASTDB 支持持久化存储与快速哈希索引
- [ ] 节点修改只改变受影响的子树 Hash，其余依赖保持不变
- [ ] 单元与集成测试全面覆盖
