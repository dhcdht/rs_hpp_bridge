use std::{fmt::format, fs, io::Write, path::{Path, PathBuf}};

use crate::gen_context::*;

pub fn gen_dart(gen_context: &GenContext, gen_out_dir: &str) {
    for hpp_element in &gen_context.hpp_elements {
        gen_dart_ffiapi(gen_context, hpp_element, gen_out_dir, None);
        gen_dart_api(gen_context, hpp_element, gen_out_dir, None);
    }
}

#[derive(Debug, Default)]
struct DartGenContext<'a> {
    pub cur_file: Option<fs::File>,
    pub cur_class: Option<&'a Class>,
}

fn gen_dart_ffiapi<'a>(gen_context: &GenContext, hpp_element: &'a HppElement, gen_out_dir: &str, ffiapi_gen_context: Option<&mut DartGenContext<'a>>) {
    match hpp_element {
        HppElement::File(file) => {
            let mut ffiapi_gen_context = DartGenContext::default();

            let hpp_filename = Path::new(&file.path).file_name().unwrap().to_os_string().into_string().unwrap();
            let dart_ffiapi_filename = hpp_filename.replace(".hpp", "_ffiapi.dart");
            let dart_ffiapi_path = PathBuf::new().join(gen_out_dir).join(dart_ffiapi_filename.clone()).into_os_string().into_string().unwrap();
            let mut ffiapi_file = fs::File::create(dart_ffiapi_path).unwrap();

            // 公共头
            let mut file_header = format!("
import 'dart:ffi';
import 'dart:io';
import 'package:ffi/ffi.dart';
            \n");
            file_header.push_str(&format!("
late final DynamicLibrary _dylib;
void {}_setDylib(DynamicLibrary dylib) {{
  _dylib = dylib;
  return;
}}
            \n", hpp_filename.replace(".hpp", "")));
            ffiapi_file.write(file_header.as_bytes());

            ffiapi_gen_context.cur_file = Some(ffiapi_file);
            for hpp_element in &file.children {
                gen_dart_ffiapi(gen_context, hpp_element, gen_out_dir, Some(&mut ffiapi_gen_context));
            }
        }
        HppElement::Class(class) => {
            let local_ffiapi_gen_context = ffiapi_gen_context.unwrap();
            
            local_ffiapi_gen_context.cur_class = Some(class);
            for hpp_element in &class.children {
                gen_dart_ffiapi(gen_context, hpp_element, gen_out_dir, Some(local_ffiapi_gen_context));
            }
            local_ffiapi_gen_context.cur_class = None;
        }
        HppElement::Method(method) => {
            let local_ffiapi_gen_context = ffiapi_gen_context.unwrap();
            let ffiapi_file = local_ffiapi_gen_context.cur_file.as_mut().unwrap();

            // 独立函数和类的函数，都走下边逻辑，需要注意区分
            let mut cur_class_name = "";
            if let Some(cur_class) = local_ffiapi_gen_context.cur_class {
                cur_class_name = &cur_class.type_str;
            }
            let is_normal_method = method.method_type == MethodType::Normal;
            let is_destructor = method.method_type == MethodType::Destructor;
            // 是否需要加第一个类的实例参数，模拟调用类实例的方法
            let need_add_first_class_param= (is_normal_method && !cur_class_name.is_empty()) || is_destructor;
            let ffiapi_c_method_name = format!("ffi_{}_{}", cur_class_name, method.name);

            // dart函数
            let mut dart_fun_decl = format!("late final {} = ptr_{}.asFunction<{} Function(",
                ffiapi_c_method_name, ffiapi_c_method_name, get_dart_fun_type_str(&method.return_type),
            );
            if need_add_first_class_param {
                dart_fun_decl.push_str("int, ");
            }
            for param in &method.params {
                dart_fun_decl.push_str(&get_dart_fun_type_str(&param.field_type));
                dart_fun_decl.push_str(", ");
            }
            if need_add_first_class_param || !method.params.is_empty() {
                dart_fun_decl.truncate(dart_fun_decl.len() - ", ".len());   // 去掉最后一个参数的, 
            }
            dart_fun_decl.push_str(")>();\n");
            ffiapi_file.write(dart_fun_decl.as_bytes());

            // native函数指针
            let mut native_fun_decl = format!("late final ptr_{} = _dylib.lookup<NativeFunction<{} Function(", 
            ffiapi_c_method_name, get_native_fun_type_str(&method.return_type));
            if need_add_first_class_param {
                native_fun_decl.push_str("Int64, ");
            }
            for param in &method.params {
                native_fun_decl.push_str(&get_native_fun_type_str(&param.field_type));
                native_fun_decl.push_str(", ");
            }
            if need_add_first_class_param || !method.params.is_empty() {
                native_fun_decl.truncate(native_fun_decl.len() - ", ".len());   // 去掉最后一个参数的, 
                
            }
            native_fun_decl.push_str(&format!(")>>('{}');\n", ffiapi_c_method_name));
            ffiapi_file.write(native_fun_decl.as_bytes());

            ffiapi_file.write("\n".as_bytes());
        }
        HppElement::Field(field) => {
            // TODO
        }
        _ => {
            unimplemented!("gen_dart_ffiapi: unknown child");
        }
    }
}

fn gen_dart_api<'a>(gen_context: &GenContext, hpp_element: &'a HppElement, gen_out_dir: &str, dart_gen_context: Option<&mut DartGenContext<'a>>) {
    match hpp_element {
        HppElement::File(file) => {
            let mut dart_gen_context = DartGenContext::default();

            let hpp_filename = Path::new(&file.path).file_name().unwrap().to_os_string().into_string().unwrap();
            let dart_ffiapi_filename = hpp_filename.replace(".hpp", "_ffiapi.dart");
            let dart_filename = hpp_filename.replace(".hpp", ".dart");
            let dart_path = PathBuf::new().join(gen_out_dir).join(dart_filename.clone()).into_os_string().into_string().unwrap();
            let mut dart_file = fs::File::create(dart_path).unwrap();

            // 公共头
            let mut file_header = format!("
import '{}';
import 'dart:ffi';
            \n", dart_ffiapi_filename);
            dart_file.write(file_header.as_bytes());

            dart_gen_context.cur_file = Some(dart_file);
            for hpp_element in &file.children {
                gen_dart_api(gen_context, hpp_element, gen_out_dir, Some(&mut dart_gen_context));
            }
        }
        HppElement::Class(class) => {
            let local_dart_gen_context = dart_gen_context.unwrap();

            // 公共头
            let dart_file_header = local_dart_gen_context.cur_file.as_mut().unwrap();
            let mut class_header = format!("
class {} {{
    late int _nativePtr;
    int getNativePtr() {{
        return _nativePtr;
    }}
            \n", 
            class.type_str);
            dart_file_header.write(class_header.as_bytes());
            
            local_dart_gen_context.cur_class = Some(class);
            for hpp_element in &class.children {
                gen_dart_api(gen_context, hpp_element, gen_out_dir, Some(local_dart_gen_context));
            }
            local_dart_gen_context.cur_class = None;

            // 公共尾
            let dart_file_footer = local_dart_gen_context.cur_file.as_mut().unwrap();
            let mut class_footer = format!("
}}
            \n", 
            );
            dart_file_footer.write(class_footer.as_bytes());
        }
        HppElement::Method(method) => {
            let local_dart_gen_context = dart_gen_context.unwrap();
            let dart_file = local_dart_gen_context.cur_file.as_mut().unwrap();

            // 独立函数和类的函数，都走下边逻辑，需要注意区分
            let mut cur_class_name = "";
            if let Some(cur_class) = local_dart_gen_context.cur_class {
                cur_class_name = &cur_class.type_str;
            }
            let ffiapi_c_method_name = format!("ffi_{}_{}", cur_class_name, method.name);
            let is_normal_method = method.method_type == MethodType::Normal;
            let is_destructor = method.method_type == MethodType::Destructor;
            // 是否需要加第一个类的实例参数，模拟调用类实例的方法
            let need_add_first_class_param= (is_normal_method && !cur_class_name.is_empty()) || is_destructor;
            
            // 函数定义
            let mut dart_fun_impl = "".to_string();
            match method.method_type {
                MethodType::Normal | MethodType::Destructor => {
                    dart_fun_impl.push_str(&format!("    {} {}(", 
                        get_dart_fun_type_str(&method.return_type), method.name));
                }
                MethodType::Constructor => {
                    dart_fun_impl.push_str(&format!("    {}.{}(", 
                        cur_class_name, method.name));
                }
                _ => {
                    unimplemented!("gen_dart_api: unknown method type")
                }
            }
            for param in &method.params {
                dart_fun_impl.push_str(&format!("{} {}, ", get_dart_fun_type_str(&param.field_type), param.name));
            }
            if !method.params.is_empty() {
                dart_fun_impl.truncate(dart_fun_impl.len() - ", ".len());   // 去掉最后一个参数的, 
            }
            dart_fun_impl.push_str(") {\n");
            // 函数实现
            match method.method_type {
                MethodType::Normal | MethodType::Destructor => {
                    dart_fun_impl.push_str(&format!("        return {}(", ffiapi_c_method_name));
                }
                MethodType::Constructor => {
                    dart_fun_impl.push_str(&format!("        _nativePtr = {}(", ffiapi_c_method_name));
                }
                _ => {
                    unimplemented!("gen_dart_api: unknown method type")
                }
            }
            if need_add_first_class_param {
                dart_fun_impl.push_str("_nativePtr, ");
            }
            for param in &method.params {
                dart_fun_impl.push_str(&format!("{}, ", param.name));
            }
            if need_add_first_class_param || !method.params.is_empty() {
                dart_fun_impl.truncate(dart_fun_impl.len() - ", ".len());   // 去掉最后一个参数的, 
            }
            dart_fun_impl.push_str(");\n    }\n");
            dart_file.write(dart_fun_impl.as_bytes());
        }
        HppElement::Field(field) => {
            // TODO
        }
        _ => {
            unimplemented!("gen_dart_api: unknown child");
        }
    }
}

fn get_dart_fun_type_str(field_type: &FieldType) -> String {
    // 基础数据类型
    if field_type.ptr_level == 0 {
        match field_type.type_kind {
            TypeKind::Void => {
                return "void".to_string();
            }
            TypeKind::Int64 => {
                return "int".to_string();
            }
            TypeKind::Float => {
                return "double".to_string();
            }
            TypeKind::Double => {
                return "double".to_string();
            }
            TypeKind::Char => {
                return "int".to_string();
            }
            _ => {
                unimplemented!("get_dart_fun_type_str: unknown type kind");
            }
        }
    }
    // class指针
    if field_type.type_kind == TypeKind::Class {
        return "int".to_string();
    }

    // 基础类型的指针
    return get_native_fun_type_str(field_type);
}

fn get_native_fun_type_str(field_type: &FieldType) -> String {
    // 基础数据类型
    if field_type.ptr_level == 0 {
        match field_type.type_kind {
            TypeKind::Void => {
                return "Void".to_string();
            }
            TypeKind::Int64 => {
                return "Int64".to_string();
            }
            TypeKind::Float => {
                return "Float".to_string();
            }
            TypeKind::Double => {
                return "Double".to_string();
            }
            TypeKind::Char => {
                return "Int8".to_string();
            }
            _ => {
                unimplemented!("get_native_fun_type_str: unknown type kind");
            }
        }
    }
    // class指针
    if field_type.type_kind == TypeKind::Class {
        return "Int64".to_string();
    }

    // 基础类型的指针
    let base_native = match field_type.type_kind {
        TypeKind::Void => "Void",
        TypeKind::Int64 => "Int64",
        TypeKind::Float => "Float",
        TypeKind::Double => "Double",
        TypeKind::Char => "Int8",
        _ => unimplemented!("get_native_fun_type_str: unknown type kind in pointer"),
    };
    let mut native_type = base_native.to_string();
    for _ in 0..field_type.ptr_level {
        native_type = format!("Pointer<{}>", native_type);
    }
    return native_type
}
