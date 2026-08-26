# 12 — 虚拟代码投影视图 (Virtual Code Projection) 与双向源码同步

**What to build:**
构建 `crates/axiom-vfs`，实现从 AST 数据库导出人类可读虚拟 `.ax` 源码的“代码投影”（Code Projection）功能，以及将外部编辑的 `.ax` 文本增量同步回 AST 数据库，实现精准哈希节点更新与无锁冲突检测。

**Blocked by:**
- 11 — AST 节点规范化哈希计算与嵌入式内容寻址存储引擎 (ASTDB)

**Status:**
ready-for-agent

**Seams:**
- `axiom_vfs::export_projection(db: &AstDatabase) -> Result<String, VfsError>`
- `axiom_vfs::sync_source_to_db(db: &mut AstDatabase, source: &str) -> Result<SyncSummary, VfsError>`

**Acceptance criteria:**
- [ ] 能从 ASTDB 完整重建出规范可编译的 `.ax` 源码
- [ ] 编辑源码后，仅重新计算并写入变更节点的 Hash，未修改节点复用已有 Hash
- [ ] 编写全面的往返 (Roundtrip) 与差异同步测试
