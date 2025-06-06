# AI 分析文档集合

本文件夹包含了使用 shrimp-task-manager 对 rs_hpp_bridge 项目进行深度分析的所有文档，用于AI上下文恢复和项目改进规划。

## 📂 文档结构

### 核心分析文档
- `01_technical_analysis.md` - 技术架构深度分析报告
- `02_task_management_guide.md` - 任务管理操作指南
- `03_project_improvement_plan.md` - 项目改进规划文档
- `04_revised_task_plan.md` - 修正后的任务规划（基于多文件支持需求）

### 任务管理快速参考
- `task_ids_reference.md` - 所有任务ID快速查找表
- `dependency_graph.md` - 任务依赖关系图表

## 🔄 上下文恢复流程

### 1. 快速了解项目状况
```bash
# 阅读技术分析报告
cat docs/ai_analysis/01_technical_analysis.md

# 查看当前任务状态
bb7_list_tasks status=all
```

### 2. 获取具体任务信息
```bash
# 查看任务详情
bb7_get_task_detail taskId="[从task_ids_reference.md获取]"

# 开始执行任务
bb7_execute_task taskId="[任务ID]"
```

### 3. 项目改进执行
参考 `03_project_improvement_plan.md` 中的路线图按优先级执行改进任务。

## ⚡ 重要更新

**最新修正** (2025年6月4日):
- 移除了对模板特化、联合体、嵌套类的支持需求
- 重点调整为多头文件支持和项目结构改进
- 任务优先级已重新调整

## 🧪 测试执行规范

**⭐ CRITICAL REMINDER FROM USER ⭐**: "不要再忘记使用 run_test.sh 运行测试了！" 

**MANDATORY**: Always use the `run_test.sh` script to execute tests. Do NOT run individual `cargo run` or `flutter test` commands manually.

**重要提醒**: 
- **必须使用项目测试脚本**: `./tests/flutter_test_project/run_test.sh`
- **禁止自行运行命令**: 不要随意执行 `cargo run`、`flutter test` 等命令
- **User has emphasized this requirement multiple times** - this is absolutely essential
- **测试脚本功能**: 完整的构建→生成→测试流程，包括多头文件桥接验证
- **测试覆盖**: 单文件桥接 + 多头文件桥接 + 跨文件引用测试

**Why this is absolutely essential:**
- Manual commands bypass critical build sequences and environment setup
- This causes incomplete or incorrect test results and wastes time
- The user has had to remind us about this requirement repeatedly

## 📝 使用说明

1. **AI首次接触**: 先阅读 `01_technical_analysis.md` 了解项目整体状况
2. **任务执行**: 使用 `02_task_management_guide.md` 中的命令操作任务系统
3. **规划参考**: 参考 `04_revised_task_plan.md` 中修正后的改进计划
4. **快速查找**: 使用 `task_ids_reference.md` 快速定位任务ID

---
**生成时间**: 2025年6月4日  
**维护者**: AI Assistant with shrimp-task-manager  
**项目**: rs_hpp_bridge C++ to Dart FFI Bridge Generator
