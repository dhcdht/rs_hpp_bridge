#!/bin/bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
PROJECT_ROOT=$( cd -- "$SCRIPT_DIR/../.." &> /dev/null && pwd )

# 项目根目录
cd "$PROJECT_ROOT"
# 构建 RsHppBridge 项目
cargo build

# 生成 bridge
RUST_BACKTRACE=1 ./target/debug/rs_hpp_bridge -i tests/flutter_test_project/src/TestModule.i -o tests/flutter_test_project/output/

# 测试项目目录
cd "$SCRIPT_DIR"
# 解决依赖
flutter pub get
# 编译native和单测目录
cd example/macos
pod install
cd ..
# 构建 native lib
flutter build macos --debug
# 运行测试
flutter test
