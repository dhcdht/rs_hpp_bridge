use std::{fs, io::Write, path::{Path, PathBuf}};

use crate::parser::{GenContext, HppElement, Class};

pub fn gen_dart(gen_context: &GenContext, gen_out_dir: &str) {
    for hpp_element in &gen_context.hpp_elements {
        gen_dart_ffiapi(gen_context, hpp_element, gen_out_dir, None);
    }
}

#[derive(Debug, Default)]
struct FFIApiGenContext<'a> {
    pub ffiapi_file: Option<fs::File>,

    pub cur_class: Option<&'a Class>,
}

fn gen_dart_ffiapi<'a>(gen_context: &GenContext, hpp_element: &'a HppElement, gen_out_dir: &str, ffiapi_gen_context: Option<&mut FFIApiGenContext<'a>>) {
    match hpp_element {
        HppElement::File(file) => {
            let mut ffiapi_gen_context = FFIApiGenContext::default();

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

            ffiapi_gen_context.ffiapi_file = Some(ffiapi_file);
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
            let mut ffiapi_file = local_ffiapi_gen_context.ffiapi_file.as_mut().unwrap();

            // 独立函数和类的函数，都走下边逻辑，需要注意区分
            let mut cur_class_name = "";
            if let Some(cur_class) = local_ffiapi_gen_context.cur_class {
                cur_class_name = &cur_class.type_str;
            }
            let ffiapi_c_method_name = format!("ffi_{}_{}", cur_class_name, method.name);

            // dart函数
            let mut dart_fun_decl = format!("late final {} = ptr_{}.asFunction<{} Function(",
                ffiapi_c_method_name, ffiapi_c_method_name, get_ffiapi_dart_fun_type_str(gen_context, &method.return_type_str),
            );
            if !cur_class_name.is_empty() {
                dart_fun_decl.push_str("int, ");
            }
            for param in &method.params {
                dart_fun_decl.push_str(&get_ffiapi_dart_fun_type_str(gen_context, &param.type_str));
                dart_fun_decl.push_str(", ");
            }
            dart_fun_decl.truncate(dart_fun_decl.len() - ", ".len());   // 去掉最后一个参数的, 
            dart_fun_decl.push_str(")>();\n");
            ffiapi_file.write(dart_fun_decl.as_bytes());

            // native函数指针
            let mut native_fun_decl = format!("late final ptr_{} = _dylib.lookup<NativeFunction<{} Function(", 
            ffiapi_c_method_name, get_ffiapi_native_fun_type_str(gen_context, &method.return_type_str));
            if !cur_class_name.is_empty() {
                native_fun_decl.push_str("Int64, ");
            }
            for param in &method.params {
                native_fun_decl.push_str(&get_ffiapi_native_fun_type_str(gen_context, &param.type_str));
                native_fun_decl.push_str(", ");
            }
            native_fun_decl.truncate(native_fun_decl.len() - ", ".len());   // 去掉最后一个参数的, 
            native_fun_decl.push_str(&format!(")>>('{}');\n", ffiapi_c_method_name));
            ffiapi_file.write(native_fun_decl.as_bytes());

            ffiapi_file.write("\n".as_bytes());
        }
        _ => {
            
        }
    }
}

fn get_ffiapi_dart_fun_type_str(gen_context: &GenContext, type_str: &str) -> String {
    match type_str {
        "void" => {
            return "void".to_string();
        }
        "int" => {
            return "int".to_string();
        }
        "float" => {
            return "double".to_string();
        }
        "double" => {
            return "double".to_string();
        }
        _ => {
            for class_name in &gen_context.class_names {
                if type_str == format!("{} *", class_name)
                || type_str == format!("{}*", class_name) {
                    return "int".to_string();
                }
            }

            return "".to_string();
        }
    }
}

fn get_ffiapi_native_fun_type_str(gen_context: &GenContext, type_str: &str) -> String {
    match type_str {
        "void" => {
            return "Void".to_string();
        }
        "int" => {
            return "Int64".to_string();
        }
        "float" => {
            return "Float".to_string();
        }
        "double" => {
            return "Double".to_string();
        }
        _ => {
            for class_name in &gen_context.class_names {
                if type_str == format!("{} *", class_name)
                || type_str == format!("{}*", class_name) {
                    return "Int64".to_string();
                }
            }

            return "".to_string();
        }
    }
}
