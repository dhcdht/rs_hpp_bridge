fn main() {
    // 仅在 macOS 上设置 rpath 以链接 Xcode 的 clang 库
    #[cfg(target_os = "macos")]
    {
        let xcode_toolchain_lib = "/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/lib";
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", xcode_toolchain_lib);
    }

    // Linux 和 Windows 通常会通过系统路径找到 clang 库，不需要额外配置
    // 如果在 Linux 上遇到链接问题，可能需要安装 libclang-dev 包
    // 如果在 Windows 上遇到问题，需要安装 LLVM 并设置环境变量
}
