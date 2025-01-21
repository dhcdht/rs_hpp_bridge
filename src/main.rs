use parser::GenContext;

mod parser;
mod gen_c;

fn main() {
    let mut gen_context = GenContext::default();
    parser::parse_hpp(&mut gen_context, "./tests/1/test.hpp");
    print!("{:#?}", gen_context);
    gen_c::gen_c(&gen_context, "./tests/1/output/");
}
