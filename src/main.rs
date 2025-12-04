use std::fs;
use clap::Parser;
use std::path::Path;

mod gen_context;
mod parser;
mod gen_c;
mod gen_dart;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// 配置文件路径
    #[arg(short, long, 
        // default_value = "",
        default_value = "tests/1/test.i",
    )]
    input: String,

    /// 输出的文件放到哪个文件夹
    #[arg(short, long, default_value = "tests/1/output")]
    outdir: String,

    /// 额外的 include 路径（可以指定多个，用冒号分隔）
    #[arg(long)]
    include_dirs: Option<String>,

    /// C++ 标准版本（如 c++11, c++14, c++17, c++20， c++23）
    #[arg(long, default_value = "c++20")]
    cpp_std: String,

    /// 额外的 clang 编译参数（可以指定多个，用空格分隔）
    #[arg(long)]
    clang_args: Option<String>,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("错误: {}", e);

        // 如果设置了 RUST_BACKTRACE 环境变量，显示更详细的信息
        if !std::env::var("RUST_BACKTRACE").is_ok() {
            eprintln!("\n提示: 设置环境变量 RUST_BACKTRACE=1 可以查看详细堆栈信息");
        }

        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse();

    // 读取输入文件
    let content = fs::read_to_string(&args.input)
        .map_err(|e| format!("无法读取输入文件 '{}': {}", args.input, e))?;

    // 解析 %include 指令
    let mut input_content_files = vec![];
    for line in content.lines() {
        if line.trim_start().starts_with("%include") && !line.trim_end().ends_with(".i\"") {
            if let (Some(start), Some(end)) = (line.find('"'), line.rfind('"')) {
                if start < end {
                    input_content_files.push(line[start + 1..end].to_string());
                }
            }
        }
    }

    if input_content_files.is_empty() {
        return Err(format!("输入文件 '{}' 中没有找到任何 %include 指令", args.input));
    }

    let input_path = Path::new(&args.input);
    let parent = input_path.parent().unwrap_or_else(|| Path::new(""));
    let h_files: Vec<_> = input_content_files.iter().map(|name| parent.join(name)).collect();

    // println!("找到 {} 个头文件需要处理", h_files.len());

    // 准备输出目录
    let gen_out_dir = &args.outdir;
    if Path::new(gen_out_dir).exists() {
        fs::remove_dir_all(gen_out_dir)
            .map_err(|e| format!("无法删除输出目录 '{}': {}", gen_out_dir, e))?;
    }
    fs::create_dir_all(gen_out_dir)
        .map_err(|e| format!("无法创建输出目录 '{}': {}", gen_out_dir, e))?;

    // 创建全局的gen_context，用于管理所有头文件的符号表
    let mut gen_context = gen_context::GenContext::default();
    let input_filename = input_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("无法获取输入文件名: {}", args.input))?;

    let module_name = input_filename
        .rfind('.')
        .map(|idx| &input_filename[..idx])
        .unwrap_or(input_filename);
    gen_context.module_name = module_name.to_string();

    // 构建 include 路径
    // 1. 默认包含 .i 文件所在的目录
    let parent_str = parent.to_str().unwrap_or("");
    let mut include_paths_vec = vec![];
    if !parent_str.is_empty() {
        include_paths_vec.push(parent_str.to_string());
    }

    // 2. 添加用户指定的额外 include 路径
    if let Some(user_dirs) = &args.include_dirs {
        for dir in user_dirs.split(':') {
            if !dir.is_empty() {
                include_paths_vec.push(dir.to_string());
            }
        }
    }

    let include_paths = include_paths_vec.join(":");

    // if !include_paths.is_empty() {
    //     println!("Include 路径: {}", include_paths);
    // }
    // println!("C++ 标准: {}", args.cpp_std);

    // 解析额外的 clang 参数
    let extra_clang_args: Vec<String> = args.clang_args
        .as_ref()
        .map(|s| s.split_whitespace().map(|s| s.to_string()).collect())
        .unwrap_or_default();

    // if !extra_clang_args.is_empty() {
    //     println!("额外 clang 参数: {}", extra_clang_args.join(" "));
    // }

    // 第一阶段：解析所有头文件，构建完整的符号表
    for h_file in &h_files {
        // println!("正在解析头文件: {:?}", h_file);
        let h_file_str = h_file.to_str()
            .ok_or_else(|| format!("无效的文件路径: {:?}", h_file))?;

        parser::parse_hpp(&mut gen_context, h_file_str, &include_paths, &args.cpp_std, &extra_clang_args);
    }

    // 第二阶段：统一生成代码
    // println!("正在生成 C 绑定代码...");
    gen_c::gen_c(&gen_context, gen_out_dir);

    // println!("正在生成 Dart 绑定代码...");
    gen_dart::gen_dart(&gen_context, gen_out_dir);

    println!("✓ 代码生成完成！输出目录: {}", gen_out_dir);
    Ok(())
}
