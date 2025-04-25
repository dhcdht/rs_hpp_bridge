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
}

fn main() {
    let args = Args::parse();
    // println!("{:#?}", args);

    let mut input_content_files = vec![];
    if !args.input.is_empty() {
        match fs::read_to_string(&args.input) {
            Ok(content) => {
                for line in content.lines() {
                    if line.trim_start().starts_with("%include") && !line.trim_end().ends_with(".i\"") {
                        let start_quote = line.find('"');
                        let end_quote = line.rfind('"');
                        if let (Some(start), Some(end)) = (start_quote, end_quote) {
                            if start < end {
                                let file_path = line[start + 1..end].to_string();
                                input_content_files.push(file_path);
                            }
                        }
                    }
                }
            }
            Err(err) => eprintln!("Error reading file {}: {}", args.input, err),
        }
    }
    let input_path = Path::new(&args.input);
    let parent = input_path.parent().unwrap_or_else(|| Path::new(""));
    let h_files: Vec<_> = input_content_files.iter().map(|name| parent.join(name)).collect();
    // println!("h_files: {:#?}", h_files);

    let gen_out_dir = &args.outdir;
    fs::remove_dir_all(gen_out_dir);
    fs::create_dir_all(gen_out_dir);
    for h_file in &h_files {
        let mut gen_context = gen_context::GenContext::default();
        parser::parse_hpp(&mut gen_context, h_file.as_path().to_str().unwrap(), parent.to_str().unwrap());
        // print!("{:#?}", gen_context);
        
        gen_c::gen_c(&gen_context, gen_out_dir);
        gen_dart::gen_dart(&gen_context, gen_out_dir);
    }
}
