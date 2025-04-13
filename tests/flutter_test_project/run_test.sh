
# 项目根目录
cd ../../
# 构建 RsHppBridge 项目
cargo build

# 生成 bridge
./target/debug/rs_hpp_bridge -i tests/flutter_test_project/src/test.i -o tests/flutter_test_project/output/

# 测试项目目录
cd tests/flutter_test_project
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
