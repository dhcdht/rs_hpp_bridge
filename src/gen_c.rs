use std::{fs, io::Write, path::{Path, PathBuf}, str::FromStr};

use crate::parser::{self, GenContext, HppElement};

pub fn gen_c(gen_context: &GenContext, gen_out_dir: &str) {
    fs::remove_dir_all(gen_out_dir);
    fs::create_dir_all(gen_out_dir);

    match gen_context.hpp_elements.first().unwrap() {
        HppElement::File(file) => {
            gen_c_file(gen_context, file, gen_out_dir);
        }
        _ => {
            
        }
    }
}

#[derive(Debug)]
struct CFileContext<'a> {
    pub ch_str: &'a mut String,
    pub cc_str: &'a mut String,

    pub gen_context: &'a GenContext,
}

fn gen_c_file(gen_context: &GenContext, file: &parser::File, gen_out_dir: &str) {
    let hpp_filename = Path::new(&file.path).file_name().unwrap().to_os_string().into_string().unwrap();
    let h_filename = hpp_filename.replace(".hpp", ".h");
    let ch_path = PathBuf::new().join(gen_out_dir).join(h_filename.clone()).into_os_string().into_string().unwrap();
    let mut ch_file = fs::File::create(ch_path).unwrap();

    let c_filename = h_filename.replace(".h", ".cpp");
    let cc_path = PathBuf::new().join(gen_out_dir).join(c_filename.clone()).into_os_string().into_string().unwrap();
    let mut cc_file = fs::File::create(cc_path).unwrap();

    let mut ch_str = String::new();
    let ch_header = r#"
#include <stdio.h>
"#;
    ch_str.push_str(&ch_header);
    let mut cc_str = String::new();
    let cc_header = format!("\n#include \"{}\"\n#include \"{}\"\n\n", h_filename, hpp_filename);
    cc_str.push_str(&cc_header);

    let mut c_context = CFileContext{
        ch_str: &mut ch_str,
        cc_str: &mut cc_str,
        gen_context: &gen_context,
    };

    for child in &file.children {
        match child {
            HppElement::Class(class) => {
                gen_c_class(&mut c_context, class);
            }
            _ => {

            }
        }
    }

    ch_file.write_all(ch_str.as_bytes());
    cc_file.write_all(cc_str.as_bytes());
}

fn gen_c_class(c_context: &mut CFileContext, class: &parser::Class) {
    let c_class_del = format!("
typedef long FFI_{};
", class.type_str);
c_context.ch_str.push_str(&c_class_del);

    for child in &class.children {
        match child {
            HppElement::Method(method) => {
                gen_c_class_method(c_context, &class, method);
            }
            _ => {

            }
        }
    }
}

fn gen_c_class_method(c_context: &mut CFileContext, class: &parser::Class, method: &parser::Method) {
    let mut method_del = format!("{} ffi_{}_{}(", method.return_type_str, class.type_str, method.name);
    method_del = replace_class_name_in_str(c_context.gen_context, &method_del);
    method_del.push_str(&format!("FFI_{} obj", class.type_str));
    for param in &method.params {
        method_del.push_str(&format!(", {} {}", param.type_str, param.name));
    }
    method_del.push_str(")");
    let mut method_impl = format!("{}", method_del);
    method_del.push_str(";\n");

    let impl_return_type = replace_class_name_in_str(c_context.gen_context, &method.return_type_str);
    let mut impl_str = format!(" {{
    {}* ptr = ({}*)obj;
    return ({})ptr->{}(", class.type_str, class.type_str, impl_return_type, method.name);
    for param in &method.params {
        impl_str.push_str(&format!("{}", param.name));
        impl_str.push_str(", ");
    }
    if !method.params.is_empty() {
        impl_str.truncate(impl_str.len() - ", ".len());
    }
    impl_str.push_str(");\n};\n");
    method_impl.push_str(&impl_str);

    c_context.ch_str.push_str(&method_del);
    c_context.cc_str.push_str(&method_impl);
}

fn replace_class_name_in_str(gen_context: &GenContext, str: &str) -> String {
    let mut ret = String::from_str(str).unwrap();
    for class_name in &gen_context.class_names {
        ret = ret.replace(&format!("{} *", class_name), &format!("FFI_{}", class_name));
        ret = ret.replace(&format!("{}*", class_name), &format!("FFI_{}", class_name));
    }

    return ret;
}
