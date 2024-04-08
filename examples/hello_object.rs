use std::io::Write;

use cranelift_codegen::Context;
use cranelift_codegen::ir::AbiParam;
use cranelift_codegen::ir::InstBuilder;

use cranelift_frontend::FunctionBuilder;
use cranelift_frontend::FunctionBuilderContext;

use cranelift_module::Module;
use cranelift_module::Linkage;
use cranelift_module::DataDescription;
use cranelift_module::default_libcall_names;

use cranelift_object::ObjectModule;
use cranelift_object::ObjectBuilder;

fn main() {
    // ------------------------------------------------------------------------
    // general setup

    let flag_builder = cranelift_codegen::settings::builder();
    let flags = cranelift_codegen::settings::Flags::new(flag_builder);
    let isa = cranelift_native::builder()
        .unwrap_or_else(|msg| {
            panic!("Error while creating Cranelift native builder: {}", msg);
        })
        .finish(flags)
        .unwrap();

    let object_name = "hello";
    let libcall_names = default_libcall_names();
    let object_builder = ObjectBuilder::new(isa, object_name, libcall_names)
        .expect("error creating object builder");

    let mut codegen_context = Context::new();
    let mut object_module = ObjectModule::new(object_builder);
    let mut func_builder_context = FunctionBuilderContext::new();

    // ------------------------------------------------------------------------
    // create the main function

    let int64 = object_module.target_config().pointer_type();

    // create a function signature for a program entry point.
    let mut sig = object_module.make_signature();
    sig.params.push(AbiParam::new(int64));
    sig.returns.push(AbiParam::new(int64));
    codegen_context.func.signature = sig;

    let mut func_builder = FunctionBuilder::new(
        &mut codegen_context.func,
        &mut func_builder_context
    );

    let entry_block = func_builder.create_block();
    func_builder.append_block_params_for_function_params(entry_block);
    func_builder.switch_to_block(entry_block);
    func_builder.seal_block(entry_block);

    // create a function signature for the `puts` function
    let mut puts_sig = object_module.make_signature();
    puts_sig.params.push(AbiParam::new(int64));
    puts_sig.returns.push(AbiParam::new(int64));

    // declare the puts function.
    // this is declared with Linkage::Import because it will be provided by the
    // system's dynamic linker
    let puts_func_id = object_module
        .declare_function("puts", Linkage::Import, &puts_sig)
        .unwrap_or_else(|msg| panic!("Failed to declaring function: {}", msg));

    // declare the puts function in the function's scope
    let puts_func_ref = object_module
        .declare_func_in_func(puts_func_id, &mut func_builder.func);

    // create a string as a global data object
    let hello_id = object_module
        .declare_data("hello_world", Linkage::Local, false, false)
        .unwrap_or_else(|msg| panic!("Error while declaring data: {}", msg));

    let hello_data = b"Hello, Sailor!\0";

    let mut data_description = DataDescription::new();
    data_description.define(hello_data.to_vec().into_boxed_slice());

    object_module.define_data(hello_id, &data_description)
        .unwrap_or_else(|msg| panic!("Error while defining data: {}", msg));

    // declare the string in the function's scope
    let hello_ptr = object_module
        .declare_data_in_func(hello_id, &mut func_builder.func);

    // get the address of the string
    let hello_value = func_builder
        .ins()
        .global_value(int64, hello_ptr);

    // call the `puts` function with the address of the string
    func_builder.ins().call(puts_func_ref, &[hello_value]);

    // return success
    let const0 = func_builder.ins().iconst(int64, 0);
    func_builder.ins().return_(&[const0]);

    func_builder.finalize();

    // ------------------------------------------------------------------------
    // write a readable version of the IR to a file
    
    let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("out")
        .join("hello_object");
    if !out_dir.exists() {
        println!("Creating out directory: {:?}", out_dir);
        std::fs::create_dir_all(&out_dir)
            .expect("error creating out directory");
    }

    let func_string = &codegen_context.func
        .display()
        .to_string();
    let mut readable_ir = std::fs::File::create("out/hello_object/hello.clir").unwrap();
    readable_ir.write_all(func_string.as_bytes())
        .expect("error writing IR to file");

    // ------------------------------------------------------------------------
    // compile the IR to a native object file

    let entry_id = object_module
        .declare_function("main", Linkage::Export, &codegen_context.func.signature)
        .unwrap_or_else(|msg| panic!("Error while declaring function: {}", msg));

    object_module
        .define_function(entry_id, &mut codegen_context)
        .unwrap_or_else(|msg| panic!("Error while defining function: {}", msg));

    object_module.clear_context(&mut codegen_context);

    let object_product = object_module.finish();
    let data_product = object_product
        .emit()
        .unwrap();

    // ------------------------------------------------------------------------
    // write the object file to disk
    
    let mut file = std::fs::File::create("out/hello_object/hello.o").unwrap();
    file.write_all(&data_product[..])
        .expect("error writing object file");


    // ------------------------------------------------------------------------
    // compile the object file to an executable

    // TODO: maybe use the cc crate to do this?
    println!("+ clang -o out/hello_object/hello.exe out/hello_object/hello.o");
    std::process::Command::new("clang")
        .arg("-o")
        .arg("out/hello_object/hello.out")
        .arg("out/hello_object/hello.o")
        .output()
        .expect("failed to execute process");
}

