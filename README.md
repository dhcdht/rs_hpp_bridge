
# 

类似 [swig](https://github.com/dhcdht/swig/tree/mobile) 的命令行工具，将 c++ 代码生成对应语言的 api。

但是 swig 有些问题
- 不支持 Flutter(Dart)
- 数据结构大量使用句柄，调试困难
- 各个语言没有共用 c++ 到 c ffi 的生成

# 支持的语言
- [WIP] Flutter(Dart)
- [ ] Java
- [ ] Obj-c
- [ ] Swift

# 特性
- [x] c++ 生成 c ffi
- [x] c++类直接生成bridge语言的类（从bridge语言调用到c++）
- [x] 回调函数（从c++调用到bridge语言）
- [x] std::string
- [WIP] 对象生命周期，上下协调一致共用
- [ ] shared_ptr
- [ ] stl
