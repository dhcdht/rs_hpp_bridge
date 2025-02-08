use std::fs;

mod gen_context;
mod parser;
mod gen_c;
mod gen_dart;

fn main() {
    let mut gen_context = gen_context::GenContext::default();
    parser::parse_hpp(&mut gen_context, "./tests/1/test.hpp");
    print!("{:#?}", gen_context);

    let gen_out_dir = "./tests/1/output/";
    fs::remove_dir_all(gen_out_dir);
    fs::create_dir_all(gen_out_dir);
    
    gen_c::gen_c(&gen_context, gen_out_dir);
    gen_dart::gen_dart(&gen_context, gen_out_dir);
}
