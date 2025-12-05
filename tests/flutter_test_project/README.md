# Flutter 集成测试

这个目录包含 rs_hpp_bridge 的完整集成测试，包括 C++ 到 Dart 的桥接代码生成和 Flutter 功能测试。

## 快速开始

### 运行完整测试（推荐）

```bash
cd tests/flutter_test_project
./run_test.sh
```

这将执行以下步骤：
1. 构建 Rust 项目
2. 生成桥接代码
3. **✨ 代码质量检查**（新增）
4. 安装 Flutter 依赖
5. 安装 CocoaPods 依赖
6. 构建 macOS 应用
7. 运行 Flutter 功能测试

### 快速质量检查模式

如果你只想快速验证代码生成质量，可以使用快速检查模式（跳过 Flutter 构建和测试）：

```bash
./run_test.sh --quick-check
# 或简写
./run_test.sh -q
```

这将只执行：
1. 构建 Rust 项目
2. 生成桥接代码
3. 代码质量检查

⏱️ **快速模式大约只需 5-10 秒**，而完整测试可能需要几分钟。

## 质量检查内容

新增的代码质量检查会自动验证：

✅ **无第三方库污染**
- 不包含 FFI_iterator、FFI_reference 等 STL 内部类型
- 不包含 FFI_json_pointer 等 JSON 库内部类型
- 不包含 FFI_longlong 等基础类型变体

✅ **无无效函数**
- Dart FFI API 中没有 ffi__has_subtype 等第三方库函数
- Dart 类中没有 has_subtype()、null() 等第三方库方法

✅ **Typedef 数量合理**
- 生成的 typedef 数量在预期范围内（5-50个）

✅ **业务类型完整**
- 所有预期的业务类型都已正确生成

## 使用示例

```bash
# 开发时快速验证
./run_test.sh -q

# 提交前完整测试
./run_test.sh

# 单独运行质量检查（需要先生成代码）
cd ../.. && cargo test --test quality_check_test
```

## 参考文档

- 完整测试改进方案：../../TESTING_IMPROVEMENTS.md
- 项目说明：../../CLAUDE.md
