#!/bin/bash
set -e  # 遇到错误立即退出

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
PROJECT_ROOT=$( cd -- "$SCRIPT_DIR/../.." &> /dev/null && pwd )

# 解析命令行参数
QUICK_CHECK=false
if [[ "$1" == "--quick-check" ]] || [[ "$1" == "-q" ]]; then
    QUICK_CHECK=true
fi

# 颜色输出
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

echo_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

echo_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检测并设置架构
detect_arch() {
    # 支持通过 FORCE_ARCH 环境变量强制指定架构
    if [[ -n "$FORCE_ARCH" ]]; then
        local arch="$FORCE_ARCH"
        echo_info "使用强制指定的架构: $arch"
    else
        local arch=$(uname -m)
        echo_info "检测到系统架构: $arch"
    fi

    if [[ "$arch" == "arm64" ]]; then
        echo_info "使用 ARM64 架构运行"
        export ARCH_PREFIX="arch -arm64"
    elif [[ "$arch" == "x86_64" ]]; then
        echo_info "使用 x86_64 架构运行"
        export ARCH_PREFIX=""
    else
        echo_warn "未知架构: $arch，尝试不指定架构"
        export ARCH_PREFIX=""
    fi
}

# 步骤 1: 构建 Rust 项目
build_rust() {
    echo_info "步骤 1/6: 构建 RsHppBridge 项目"
    cd "$PROJECT_ROOT"
    if ! cargo build; then
        echo_error "Rust 项目构建失败"
        exit 1
    fi
    echo_info "✓ Rust 项目构建成功"
}

# 步骤 2: 生成桥接代码
generate_bridge() {
    echo_info "步骤 2/7: 生成桥接代码"
    cd "$PROJECT_ROOT"
    if ! RUST_BACKTRACE=1 ./target/debug/rs_hpp_bridge \
        -i tests/flutter_test_project/src/TestModule.i \
        -o tests/flutter_test_project/output/; then
        echo_error "桥接代码生成失败"
        exit 1
    fi
    echo_info "✓ 桥接代码生成成功"
}

# 步骤 3: 运行质量检查测试
quality_check() {
    echo_info "步骤 3/7: 运行代码质量检查"
    cd "$PROJECT_ROOT"
    if ! cargo test --test quality_check_test -- --nocapture; then
        echo_error "代码质量检查失败"
        echo_error "生成的代码可能包含第三方库污染或其他质量问题"
        echo_error "请检查测试输出以了解详细信息"
        exit 1
    fi
    echo_info "✓ 代码质量检查通过"
}

# 步骤 4: 安装 Flutter 依赖
install_flutter_deps() {
    echo_info "步骤 4/7: 安装 Flutter 依赖"
    cd "$SCRIPT_DIR"
    if ! $ARCH_PREFIX fvm flutter pub get; then
        echo_error "Flutter 依赖安装失败"
        exit 1
    fi
    echo_info "✓ Flutter 依赖安装成功"
}

# 步骤 5: 安装 CocoaPods 依赖
install_pod_deps() {
    echo_info "步骤 5/7: 安装 CocoaPods 依赖"
    cd "$SCRIPT_DIR/example/macos"

    # 根据架构设置 CocoaPods 的架构配置
    if [[ "$ARCH_PREFIX" == "arch -arm64" ]]; then
        # arm64 架构：在 Podfile 中排除 x86_64
        export ARCHS="arm64"
    fi

    if ! $ARCH_PREFIX pod install; then
        echo_warn "CocoaPods 安装有警告，但继续执行..."
    fi
    echo_info "✓ CocoaPods 依赖处理完成"
}

# 步骤 6: 构建 macOS 应用
build_macos() {
    echo_info "步骤 6/7: 构建 macOS 应用"
    cd "$SCRIPT_DIR/example"

    # 根据架构设置不同的构建配置
    if [[ "$ARCH_PREFIX" == "arch -arm64" ]]; then
        # arm64 架构：清理之前的构建产物并强制 arm64 架构
        echo_info "清理之前的 x86_64 构建产物..."
        $ARCH_PREFIX fvm flutter clean

        # 设置 Xcode 环境变量强制使用 arm64 架构
        export ARCHS="arm64"
        export ONLY_ACTIVE_ARCH="NO"
        export VALID_ARCHS="arm64"

        if ! $ARCH_PREFIX fvm flutter build macos --debug; then
            echo_error "macOS 应用构建失败"
            exit 1
        fi
    else
        # x86_64 架构或默认架构
        if ! $ARCH_PREFIX fvm flutter build macos --debug; then
            echo_error "macOS 应用构建失败"
            exit 1
        fi
    fi

    echo_info "✓ macOS 应用构建成功"
}

# 步骤 7: 运行 Flutter 功能测试
run_tests() {
    echo_info "步骤 7/7: 运行 Flutter 功能测试"
    cd "$SCRIPT_DIR/example"
    if ! $ARCH_PREFIX fvm flutter test; then
        echo_error "Flutter 功能测试失败"
        exit 1
    fi
    echo_info "✓ 所有 Flutter 功能测试通过"
}

# 快速检查模式：只执行构建、生成和质量检查
quick_check_mode() {
    echo_info "========================================="
    echo_info "  快速质量检查模式"
    echo_info "========================================="

    detect_arch
    build_rust
    generate_bridge
    quality_check

    echo_info "========================================="
    echo_info "  ✓ 快速检查完成！"
    echo_info "  - Rust 项目构建成功"
    echo_info "  - 桥接代码生成成功"
    echo_info "  - 代码质量检查通过"
    echo_info ""
    echo_info "  提示: 运行完整测试请执行 ./run_test.sh"
    echo_info "========================================="
}

# 主流程
main() {
    echo_info "========================================="
    echo_info "  Flutter 集成测试脚本"
    echo_info "========================================="

    detect_arch
    build_rust
    generate_bridge
    quality_check       # 新增：代码质量检查
    install_flutter_deps
    install_pod_deps
    build_macos
    run_tests

    echo_info "========================================="
    echo_info "  ✓ 所有步骤完成！"
    echo_info "  - Rust 项目构建成功"
    echo_info "  - 桥接代码生成成功"
    echo_info "  - 代码质量检查通过"
    echo_info "  - Flutter 功能测试通过"
    echo_info "========================================="
}

# 根据参数选择运行模式
if [ "$QUICK_CHECK" = true ]; then
    quick_check_mode
else
    main
fi
