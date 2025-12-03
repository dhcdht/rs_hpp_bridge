# CLAUDE.md

本文件为 Claude Code (claude.ai/code) 提供在此仓库中工作的指导。

## 项目概述

rs_hpp_bridge 是一个命令行代码生成工具，用于为 C++ 代码创建语言绑定，类似于 SWIG，但对 Flutter/Dart 有更好的支持。它解析 C++ 头文件并生成：
- C FFI 包装代码（`.cpp` 和 `.h` 文件）
- 目标语言绑定（当前支持 Dart，计划支持 Java/Obj-C/Swift）

该工具解决了 SWIG 的局限性：不支持 Flutter、大量使用句柄导致调试困难、各语言没有共用 C++ 到 C FFI 的生成。

## 构建和运行命令

### 构建项目
```bash
cargo build
```

### Release 模式构建
```bash
cargo build --release
```

### 运行代码生成器
```bash
# 基本用法
./target/debug/rs_hpp_bridge -i <input.i> -o <output_dir>

# 使用测试项目的示例
./target/debug/rs_hpp_bridge \
  -i tests/flutter_test_project/src/TestModule.i \
  -o tests/flutter_test_project/output/
```

### 运行测试
```bash
# 运行所有测试
cargo test

# 运行特定测试
cargo test test_parse_hpp

# 带堆栈跟踪运行
RUST_BACKTRACE=1 cargo test
```

### 运行 Flutter 集成测试
```bash
cd tests/flutter_test_project
./run_test.sh
```

此脚本会构建 Rust 项目、生成绑定代码、构建 Flutter native 库并运行 Flutter 测试。

## 架构

### 三阶段代码生成

1. **解析阶段** (`parser.rs`)
   - 使用 `clang` crate 解析 C++ 头文件
   - 从 libclang 实体构建 AST（抽象语法树）
   - 处理类、方法、构造函数、析构函数、函数、字段和参数
   - 过滤系统头文件

2. **上下文构建阶段** (`gen_context.rs`)
   - 将所有解析的元素收集到 `GenContext` 结构中
   - 维护跨所有头文件的符号表
   - 识别特殊类型：回调、STL 容器（vector、map、set）、shared_ptr

3. **代码生成阶段**
   - **C FFI 生成** (`gen_c.rs`)：生成 `*_ffi.cpp` 和 `*_ffi.h` 文件，包含 C 包装函数
   - **Dart 生成** (`gen_dart.rs`)：生成 `*.dart`、`*_ffiapi.dart` 文件，包含 Dart 类绑定

### 输入文件格式

工具接受 `.i` 接口文件（类似 SWIG），列出要处理的头文件：

```
%include "simple_types.hpp"
%include "simple_a.hpp"
%include "simple_b.hpp"
```

只有使用 `%include` 指令列出的文件（不以 `.i"` 结尾）会被处理。

### 关键数据结构

- `GenContext`：全局上下文，包含模块名和所有解析的 HPP 元素
- `HppElement`：枚举，表示 File、Class、Method 或 Field
- `Class`：表示 C++ 类，包含类型信息、子元素和特殊标志（ClassType: Normal、Callback、StdPtr、StdVector、StdMap 等）
- `Method`：包含名称、返回类型、参数、静态标志和方法类型（Normal、Constructor、Destructor）
- `FieldType`：表示 C++ 类型，包含 TypeKind（Void、Int64、Float、Double、String 等）和修饰符（const、引用、指针）

### 支持的 C++ 特性

- C++ 类映射到 Dart 类
- 异步回调函数（从 C++ 调用到 Dart）
- std::string
- struct 类型
- C++ 和 Dart 之间的对象生命周期管理
- shared_ptr
- STL 容器：std::vector、std::map、std::unordered_map、std::set、std::unordered_set

## 开发注意事项

### 平台特定配置

`build.rs` 文件包含 macOS 特定的 rpath 配置，用于链接 Xcode 的 clang 库。在其他平台上可能需要调整。

### 解析器实现

解析器通过 `visit_parse_clang_entity()` 递归遍历 libclang 实体。关键处理函数：
- `handle_clang_ClassDecl`：处理类定义
- `handle_clang_Method`：处理成员函数
- `handle_clang_Constructor`/`handle_clang_Destructor`：特殊方法处理
- `handle_clang_FunctionDecl`：独立函数
- `handle_clang_ParmDecl`：函数参数
- `handle_clang_FieldDecl`：类字段

### 代码生成策略

每个生成器为每个输入头文件创建文件：
- C：`<filename>_ffi.h` 和 `<filename>_ffi.cpp`
- Dart：`<filename>.dart` 和 `<filename>_ffiapi.dart`

生成一个公共文件：`<module_name>_public.dart`，包含动态库初始化代码。

### 测试结构

- `tests/parser_test/`：解析器单元测试，期望输出在 `ut_result/` 中
- `tests/flutter_test_project/`：集成测试，包含 Flutter 插件，演示端到端工作流
- `tests/java_test_project/`：为未来 Java 支持预留的占位符

### 类型名称简化

函数名称从 C++ 类型生成，使用 `simplify_type_for_naming()` 函数，该函数移除 const、引用、指针，并替换特殊字符以创建有效的标识符。
