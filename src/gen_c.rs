use std::{fs, io::Write, path::{Path, PathBuf}, str::FromStr};

use crate::gen_context::*;

pub fn gen_c(gen_context: &GenContext, gen_out_dir: &str) {
    match gen_context.hpp_elements.first().unwrap() {
        HppElement::File(file) => {
            gen_c_file(gen_context, file, gen_out_dir);
        }
        _ => {
            unimplemented!("gen_c: first element is not File");
        }
    }
}

#[derive(Debug)]
struct CFileContext<'a> {
    pub ch_str: &'a mut String,
    pub cc_str: &'a mut String,

    pub gen_context: &'a GenContext,
}

fn gen_c_file(gen_context: &GenContext, file: &File, gen_out_dir: &str) {
    let hpp_filename = Path::new(&file.path).file_name().unwrap().to_os_string().into_string().unwrap();
    let h_filename = hpp_filename.replace(".hpp", "_ffi.h");
    let ch_path = PathBuf::new().join(gen_out_dir).join(h_filename.clone()).into_os_string().into_string().unwrap();
    let mut ch_file = fs::File::create(ch_path).unwrap();

    let c_filename = h_filename.replace(".h", ".cpp");
    let cc_path = PathBuf::new().join(gen_out_dir).join(c_filename.clone()).into_os_string().into_string().unwrap();
    let mut cc_file = fs::File::create(cc_path).unwrap();

    let mut ch_str = String::new();
    // 公共头
    let ch_header = r#"
#include <stdio.h>

extern "C" {
"#;
    ch_str.push_str(&ch_header);
    let mut cc_str = String::new();
    let cc_header = format!("
#include \"{}\"
#include \"{}\"

extern \"C\" {{

", h_filename, hpp_filename);
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
                unimplemented!("gen_c_file: unknown child");
            }
        }
    }

    // 公共尾
    let ch_footer = r#"
} // extern "C"
"#;
    ch_str.push_str(&ch_footer);
    let cc_footer = r#"
} // extern "C"
"#;
    cc_str.push_str(&cc_footer);

    ch_file.write_all(ch_str.as_bytes());
    cc_file.write_all(cc_str.as_bytes());
}

fn gen_c_class(c_context: &mut CFileContext, class: &Class) {
    let c_class_decl = format!("
typedef long FFI_{};
", class.type_str);
c_context.ch_str.push_str(&c_class_decl);

    for child in &class.children {
        match child {
            HppElement::Method(method) => {
                gen_c_class_method(c_context, Some(&class), method);
            }
            HppElement::Field(field) => {
                // TODO
            }
            _ => {
                unimplemented!("gen_c_class: unknown child");
            }
        }
    }
}

fn gen_c_class_method(c_context: &mut CFileContext, class: Option<&Class>, method: &Method) {
    let mut cur_class_name = "";
    if let Some(cur_class) = class {
        cur_class_name = &cur_class.type_str;
    }
    let is_normal_method = method.method_type == MethodType::Normal;
    let is_destructor = method.method_type == MethodType::Destructor;
    // 是否需要加第一个类的实例参数，模拟调用类实例的方法
    let need_add_first_class_param= (is_normal_method && !cur_class_name.is_empty()) || is_destructor;

    let mut method_decl = format!("{} ffi_{}_{}(", get_ffi_type_str(&method.return_type), cur_class_name, method.name);
    if need_add_first_class_param {
        method_decl.push_str(&format!("FFI_{} obj, ", cur_class_name));
    }
    for param in &method.params {
        method_decl.push_str(&format!("{} {}, ", get_ffi_type_str(&param.field_type), param.name));
    }
    if need_add_first_class_param || !method.params.is_empty() {
        method_decl.truncate(method_decl.len() - ", ".len()); // 去掉最后一个参数的, 
    }
    method_decl.push_str(")");
    let mut method_impl = format!("{}", method_decl);
    method_decl.push_str(";\n");

    let impl_return_type = get_ffi_type_str(&method.return_type);
    let mut impl_str: String = "".to_string();
    match method.method_type {
        MethodType::Normal => {
            // 普通类函数
            if !cur_class_name.is_empty() {
                impl_str.push_str(&format!(" {{
    {}* ptr = ({}*)obj;
    return ({})ptr->{}(", 
                    cur_class_name, cur_class_name, impl_return_type, method.name));
            } 
            // 独立函数
            else {
                impl_str.push_str(&format!(" {{
    return ({}){}(", 
                    impl_return_type, method.name));
            }
        }
        // 构造函数
        MethodType::Constructor => {
            impl_str.push_str(&format!(" {{
    return ({})new {}(", 
                impl_return_type, cur_class_name));
        }
        MethodType::Destructor => {
            impl_str.push_str(&format!(" {{
    {}* ptr = ({}*)obj;
    return delete ptr; //", 
                cur_class_name, cur_class_name));
        }
        _ => {
            unimplemented!("gen_c_class_method: unknown method type");
        }
    }
    for param in &method.params {
        impl_str.push_str(&format!("({}){}, ", &param.field_type.full_str, param.name));
    }
    if !method.params.is_empty() {
        impl_str.truncate(impl_str.len() - ", ".len()); // 去掉最后一个参数的, 
    }
    impl_str.push_str(");\n};\n");
    method_impl.push_str(&impl_str);

    c_context.ch_str.push_str(&method_decl);
    c_context.cc_str.push_str(&method_impl);
}

fn get_ffi_type_str(field_type: &FieldType) -> String {
    match field_type.type_kind {
        TypeKind::Class => {
            return format!("FFI_{}", field_type.type_str);
        }
        _ => {
            return field_type.full_str.clone();
        }
    }
}
