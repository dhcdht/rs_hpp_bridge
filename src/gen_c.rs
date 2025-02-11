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
    let mut ch_header = format!("
#include <stdio.h>

extern \"C\" {{
");
    for class_name in &gen_context.class_names {
        ch_header.push_str(&format!("typedef long FFI_{};\n", class_name));
    }
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
                if class.is_callback() {
                    gen_c_callback_class(&mut c_context, class);
                } else {
                    gen_c_class(&mut c_context, class);
                }
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
    let c_class_decl = format!("\n");
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
                if method.return_type.type_kind == TypeKind::String {
                    impl_str.push_str(&format!(" {{
    {}* ptr = ({}*)obj;
    static std::string retStr = ptr->{}(", 
                    cur_class_name, cur_class_name, method.name));
                } 
                else {
                    impl_str.push_str(&format!(" {{
    {}* ptr = ({}*)obj;
    return ({})ptr->{}(", 
                    cur_class_name, cur_class_name, impl_return_type, method.name));
                }
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
        if param.field_type.type_kind == TypeKind::String {
            impl_str.push_str(&format!("std::string({}), ", param.name));
        } else {
            impl_str.push_str(&format!("({}){}, ", &param.field_type.full_str, param.name));
        }
    }
    if !method.params.is_empty() {
        impl_str.truncate(impl_str.len() - ", ".len()); // 去掉最后一个参数的, 
    }
    if method.return_type.type_kind == TypeKind::String {
        impl_str.push_str(");\n    return (const char*)retStr.c_str();\n};\n");
    } else {
        impl_str.push_str(");\n};\n");
        
    }
    method_impl.push_str(&impl_str);

    c_context.ch_str.push_str(&method_decl);
    c_context.cc_str.push_str(&method_impl);
}

/// 回调类
fn gen_c_callback_class(c_context: &mut CFileContext, class: &Class) {
    let c_class_callback_decl = format!("\n");
    c_context.ch_str.push_str(&c_class_callback_decl);

    // 子类化的回调类的名字
    let subclass_name = format!("Impl_{}", class.type_str);

    // 生成注册函数的定义
    for child in &class.children {
        match child {
            HppElement::Method(method) => {
                // 这次只生成需要重写的回调函数
                if method.method_type == MethodType::Normal {
                    gen_c_callback_regist_decl(c_context, Some(&class), method);
                }
            }
            HppElement::Field(field) => {
                // TODO
            }
            _ => {
                unimplemented!("gen_c_callback_class: unknown child");
            }
        }
    }

    // 生成回调子类
    let mut c_class_callback_impl = format!("class {} : {} {{
public:
", subclass_name, class.type_str);
c_context.cc_str.push_str(&c_class_callback_impl);
    for child in &class.children {
        match child {
            HppElement::Method(method) => {
                // 这次只生成需要重写的回调函数
                if method.method_type == MethodType::Normal {
                    gen_c_callback_class_method(c_context, Some(&class), method);
                }
            }
            HppElement::Field(field) => {
                // TODO
            }
            _ => {
                unimplemented!("gen_c_callback_class: unknown child");
            }
        }
    }
    c_context.cc_str.push_str("\n};\n");

    // 生成回调子类的其他正常函数
    for child in &class.children {
        match child {
            HppElement::Method(method) => {
                gen_c_class_method(c_context, Some(&class), method);
                // 作为回调的抽象类并不能new，所以这里换成可实例化的子类
                let form_new = format!("new {}", class.type_str);
                let to_new = format!("new {}", subclass_name);
                if let Some(pos) = c_context.cc_str.find(&form_new) {
                    let end = pos + form_new.len();
                    c_context.cc_str.replace_range(pos..end, &to_new);
                }
            }
            HppElement::Field(field) => {
                // TODO
            }
            _ => {
                unimplemented!("gen_c_callback_class: unknown child");
            }
        }
    }

    // 生成注册函数的实现
    for child in &class.children {
        match child {
            HppElement::Method(method) => {
                // 这次只生成需要重写的回调函数
                if method.method_type == MethodType::Normal {
                    gen_c_callback_regist_impl(c_context, Some(&class), method);
                }
            }
            HppElement::Field(field) => {
                // TODO
            }
            _ => {
                unimplemented!("gen_c_callback_class: unknown child");
            }
        }
    }

    c_context.cc_str.push_str("\n");
}

/// 回调类的方法
fn gen_c_callback_class_method(c_context: &mut CFileContext, class: Option<&Class>, method: &Method) {
    if (method.method_type == MethodType::Constructor) || (method.method_type == MethodType::Destructor) {
        return;
    }

    // ffi 中的类型名
    let ffi_class_name = format!("FFI_{}", class.unwrap().type_str);
    // 函数指针类型的名字
    let fun_ptr_type_str = format!("{}_{}", ffi_class_name, method.name);
    // 指向函数指针的变量
    let fun_ptr_var_str = format!("{}_{}", class.unwrap().type_str, method.name);

    // .cpp 中的实现
    // 调用函数指针的函数实现
    let mut c_class_callback_method_impl = format!("    virtual {} {}(", 
        method.return_type.full_str, method.name);
    for param in &method.params {
        c_class_callback_method_impl.push_str(&format!("{} {}, ", param.field_type.full_str, param.name));
    }
    if !method.params.is_empty() {
        c_class_callback_method_impl.truncate(c_class_callback_method_impl.len() - ", ".len()); // 去掉最后一个参数的, 
    }
    c_class_callback_method_impl.push_str(&format!(") override {{
        return {}(({})this, ", 
        fun_ptr_var_str, ffi_class_name));
    for param in &method.params {
        c_class_callback_method_impl.push_str(&format!("({}){}, ", get_ffi_type_str(&param.field_type), param.name));
    }
    if !method.params.is_empty() {
        c_class_callback_method_impl.truncate(c_class_callback_method_impl.len() - ", ".len()); // 去掉最后一个参数的, 
    }
    c_class_callback_method_impl.push_str(");\n\t};\n");
    c_context.cc_str.push_str(&c_class_callback_method_impl);
}

/// 生成注册函数的定义
fn gen_c_callback_regist_decl(c_context: &mut CFileContext, class: Option<&Class>, method: &Method) {
    if (method.method_type == MethodType::Constructor) || (method.method_type == MethodType::Destructor) {
        return;
    }

    // ffi 中的类型名
    let ffi_class_name = format!("FFI_{}", class.unwrap().type_str);
    // 函数指针类型的名字
    let fun_ptr_type_str = format!("{}_{}", ffi_class_name, method.name);
    // 指向函数指针的变量
    let fun_ptr_var_str = format!("{}_{}", class.unwrap().type_str, method.name);

    // .h 中的声明
    // 1. 函数指针类型声明
    let mut c_class_callback_method_decl = format!("typedef {} (*{})(", 
        get_ffi_type_str(&method.return_type), fun_ptr_type_str);
    c_class_callback_method_decl.push_str(&format!("{} obj, ", ffi_class_name));
    for param in &method.params {
        c_class_callback_method_decl.push_str(&format!("{} {}, ", get_ffi_type_str(&param.field_type), param.name));
    }
    if !method.params.is_empty() {
        c_class_callback_method_decl.truncate(c_class_callback_method_decl.len() - ", ".len()); // 去掉最后一个参数的, 
    }
    c_class_callback_method_decl.push_str(");\n");
    // 2. 注册函数指针的函数声明
    c_class_callback_method_decl.push_str(&format!("void {}_regist({} {});\n"
        , fun_ptr_type_str, fun_ptr_type_str, method.name));
    c_context.ch_str.push_str(&c_class_callback_method_decl);

    // .cpp 中的实现
    // 1. 注册函数指针的实现
    let fun_ptr_decl = format!("static {} {} = nullptr;\n", fun_ptr_type_str, fun_ptr_var_str);
    c_context.cc_str.push_str(&fun_ptr_decl);
}

/// 生成注册函数的实现
fn gen_c_callback_regist_impl(c_context: &mut CFileContext, class: Option<&Class>, method: &Method) {
    if (method.method_type == MethodType::Constructor) || (method.method_type == MethodType::Destructor) {
        return;
    }

    // ffi 中的类型名
    let ffi_class_name = format!("FFI_{}", class.unwrap().type_str);
    // 函数指针类型的名字
    let fun_ptr_type_str = format!("{}_{}", ffi_class_name, method.name);
    // 指向函数指针的变量
    let fun_ptr_var_str = format!("{}_{}", class.unwrap().type_str, method.name);

    let mut fun_ptr_impl = format!("void {}_regist({} {}){{
    {} = {};
}};
", fun_ptr_type_str, fun_ptr_type_str, method.name, fun_ptr_var_str, method.name);
    c_context.cc_str.push_str(&fun_ptr_impl);
}

fn get_ffi_type_str(field_type: &FieldType) -> String {
    match field_type.type_kind {
        TypeKind::Void | TypeKind::Int64 | TypeKind::Float | TypeKind::Double | TypeKind::Char => {
            return field_type.full_str.clone();
        }
        TypeKind::String => {
            return "const char*".to_string();
        }
        TypeKind::Class => {
            return format!("FFI_{}", field_type.type_str);
        }
        _ => {
            unimplemented!("get_ffi_type_str: unknown type kind");
        }
    }
}
