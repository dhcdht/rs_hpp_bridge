use std::{fs, io::Write, path::{Path, PathBuf}, str::FromStr};

use crate::parser::{self, GenContext, HppElement};

pub fn gen_c(gen_context: &GenContext, gen_out_dir: &str) {
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
    let h_filename = hpp_filename.replace(".hpp", "_ffi.h");
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
            // 独立函数
            HppElement::Method(method) => {
                gen_c_class_method(&mut c_context, None, method);
            }
            _ => {

            }
        }
    }

    ch_file.write_all(ch_str.as_bytes());
    cc_file.write_all(cc_str.as_bytes());
}

fn gen_c_class(c_context: &mut CFileContext, class: &parser::Class) {
    let c_class_decl = format!("
typedef long FFI_{};
", class.type_str);
c_context.ch_str.push_str(&c_class_decl);

    for child in &class.children {
        match child {
            HppElement::Method(method) => {
                gen_c_class_method(c_context, Some(&class), method);
            }
            _ => {

            }
        }
    }
}

fn gen_c_class_method(c_context: &mut CFileContext, class: Option<&parser::Class>, method: &parser::Method) {
    let mut cur_class_name = "";
    if let Some(cur_class) = class {
        cur_class_name = &cur_class.type_str;
    }

    let mut method_decl = format!("{} ffi_{}_{}(", method.return_type_str, cur_class_name, method.name);
    if !cur_class_name.is_empty() {
        method_decl.push_str(&format!("FFI_{} obj, ", cur_class_name));
    }
    for param in &method.params {
        method_decl.push_str(&format!("{} {}", param.type_str, param.name));
        method_decl.push_str(", ");
    }
    method_decl.truncate(method_decl.len() - ", ".len()); // 去掉最后一个参数的, 
    method_decl = replace_class_to_ffi_str(c_context.gen_context, &method_decl);
    method_decl.push_str(")");
    let mut method_impl = format!("{}", method_decl);
    method_decl.push_str(";\n");

    let impl_return_type = replace_class_to_ffi_str(c_context.gen_context, &method.return_type_str);
    let mut impl_str: String = "".to_string();
    if !cur_class_name.is_empty() {
        impl_str.push_str(&format!(" {{
    {}* ptr = ({}*)obj;
    return ({})ptr->{}(", 
            cur_class_name, cur_class_name, impl_return_type, method.name));
    } else {
        impl_str.push_str(&format!(" {{
    return ({}){}(", 
            impl_return_type, method.name));
    }
    for param in &method.params {
        impl_str.push_str(&format!("({}){}", replace_ffi_to_class_str(c_context.gen_context, &param.type_str), param.name));
        impl_str.push_str(", ");
    }
    if !method.params.is_empty() {
        impl_str.truncate(impl_str.len() - ", ".len()); // 去掉最后一个参数的, 
    }
    impl_str.push_str(");\n};\n");
    method_impl.push_str(&impl_str);

    c_context.ch_str.push_str(&method_decl);
    c_context.cc_str.push_str(&method_impl);
}

fn replace_class_to_ffi_str(gen_context: &GenContext, str: &str) -> String {
    let mut ret = String::from_str(str).unwrap();
    for class_name in &gen_context.class_names {
        ret = ret.replace(&format!("{} *", class_name), &format!("FFI_{}", class_name));
        ret = ret.replace(&format!("{}*", class_name), &format!("FFI_{}", class_name));
    }

    return ret;
}

fn replace_ffi_to_class_str(gen_context: &GenContext, str: &str) -> String {
    let mut ret = String::from_str(str).unwrap();
    for class_name in &gen_context.class_names {
        ret = ret.replace(&format!("FFI_{}", class_name), &format!("{}*", class_name));
    }

    return ret;
}
