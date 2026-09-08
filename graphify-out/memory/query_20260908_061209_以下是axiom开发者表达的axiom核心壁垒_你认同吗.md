---
type: "query"
date: "2026-09-08T06:12:09.356208+00:00"
question: "以下是Axiom开发者表达的Axiom核心壁垒，你认同吗？"
contributor: "graphify"
outcome: "useful"
source_nodes: ["AgentTaskPool", "AstDatabase", "SmtResult", "eval_runner.py", "extract_holes()", "verify()", "twenty_workers_verify_and_publish_a_hole_free_release()"]
---

# Q: 以下是Axiom开发者表达的Axiom核心壁垒，你认同吗？

## Answer

不认同绝对化结论。最新版本的严格AgentPool、跨函数SMT和代数效应转译是实质进步，但当前数据只测输入prompt token，generate模式没有编译、验证或重试循环；Group 3还省略了业务描述并使用不足以定义业务语义的弱契约。纯函数和契约能降低冲突与缺陷概率，却不能保证零语义冲突或零业务Bug。当前实现仍有Raw语法逃逸、默认verify接受Sampled、内容寻址数据库无Root Ref合并协议等边界。其他语言可借助Dafny、F*、Kani、Creusot和Unison实现相同类别能力，因此真正差异是集成工作流，不是理论不可复制的护城河。

## Outcome

- Signal: useful

## Source Nodes

- AgentTaskPool
- AstDatabase
- SmtResult
- eval_runner.py
- extract_holes()
- verify()
- twenty_workers_verify_and_publish_a_hole_free_release()