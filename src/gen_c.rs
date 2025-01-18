use std::{fs, io::{Read, Write}, path::{Path, PathBuf}};

use crate::parser::{self, HppElement};

pub fn gen_c(hpp: parser::HppElement, out_dir: &str) {
    fs::remove_dir_all(out_dir);
    fs::create_dir_all(out_dir);

    match hpp {
        HppElement::File(file) => {
            gen_c_file(file, out_dir);
        }
        _ => {
            
        }
    }
}

#[derive(Debug)]
struct GenFileContext<'a> {
    pub ch_str: &'a mut String,
    pub cc_str: &'a mut String,
}

fn gen_c_file(file: parser::File, out_dir: &str) {
    let hpp_filename = Path::new(&file.path).file_name().unwrap().to_os_string().into_string().unwrap();
    let h_filename = hpp_filename.replace(".hpp", ".h");
    let ch_path = PathBuf::new().join(out_dir).join(h_filename.clone()).into_os_string().into_string().unwrap();
    let mut ch_file = fs::File::create(ch_path).unwrap();

    let c_filename = h_filename.replace(".h", ".c");
    let cc_path = PathBuf::new().join(out_dir).join(c_filename.clone()).into_os_string().into_string().unwrap();
    let mut cc_file = fs::File::create(cc_path).unwrap();

    let mut ch_str = String::new();
    let ch_header = r#"
#include <stdio.h>
"#;
    ch_str.push_str(&ch_header);
    let mut cc_str = String::new();
    let cc_header = format!("\n#include \"{}\";\n#include \"{}\";\n\n", h_filename, hpp_filename);
    cc_str.push_str(&cc_header);

    let mut gen_context = GenFileContext{
        ch_str: &mut ch_str,
        cc_str: &mut cc_str,
    };

    for child in &file.children {
        match child {
            HppElement::Class(class) => {
                gen_c_class(&mut gen_context, class);
            }
            _ => {

            }
        }
    }

    ch_file.write_all(ch_str.as_bytes());
    cc_file.write_all(cc_str.as_bytes());
}

fn gen_c_class(gen_context: &mut GenFileContext, class: &parser::Class) {
    let c_class_del = format!("
typedef int FFI_{};
", class.typeStr);
    gen_context.ch_str.push_str(&c_class_del);

    for child in &class.children {
        match child {
            HppElement::Method(method) => {
                gen_c_class_method(gen_context, &class, method);
            }
            _ => {

            }
        }
    }
}

fn gen_c_class_method(gen_context: &mut GenFileContext, class: &parser::Class, method: &parser::Method) {
    let mut method_del = format!("{} ffi_{}(", method.returnTypeStr, method.name);
    method_del.push_str(&format!("FFI_{} obj", class.typeStr));
    for param in &method.params {
        method_del.push_str(&format!(", {} {}", param.typeStr, param.name));
    }
    method_del.push_str(")");
    let mut method_impl = format!("{}", method_del);
    method_del.push_str(";\n");

    let mut impl_str = format!(" {{
    {}* ptr = ({}*)obj;
    return ({})ptr->{}(", class.typeStr, class.typeStr, method.returnTypeStr, method.name);
    for param in &method.params {
        impl_str.push_str(&format!("{} {}", param.typeStr, param.name));
        impl_str.push_str(", ");
    }
    if !method.params.is_empty() {
        impl_str.truncate(impl_str.len() - ", ".len());
    }
    impl_str.push_str(");\n};\n");
    method_impl.push_str(&impl_str);

    gen_context.ch_str.push_str(&method_del);
    gen_context.cc_str.push_str(&method_impl);
}
