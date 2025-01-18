mod parser;
mod gen_c;

fn main() {
    let hpp = parser::parse_hpp("./tests/1/test.hpp");
    gen_c::gen_c(hpp, "./tests/1/output/");
}
