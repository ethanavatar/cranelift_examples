
use std::io::Write;

use cranelift_codegen::Context;
use cranelift_codegen::ir::AbiParam;
use cranelift_codegen::ir::InstBuilder;

use cranelift_frontend::FunctionBuilder;
use cranelift_frontend::FunctionBuilderContext;

use cranelift_jit::JITBuilder;
use cranelift_jit::JITModule;
use cranelift_module::Module;
use cranelift_module::Linkage;
use cranelift_module::DataDescription;

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

    let libcall_names = cranelift_module::default_libcall_names();
    let jit_builder = JITBuilder::with_isa(isa, libcall_names);

    let mut jit_module = JITModule::new(jit_builder);
    let mut codegen_context = jit_module.make_context();
    let mut func_builder_context = FunctionBuilderContext::new();

    // ------------------------------------------------------------------------
    // create the main function

    let int64 = jit_module.target_config().pointer_type();

    // create a function signature for a program entry point.
    let mut sig = jit_module.make_signature();
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
    let mut puts_sig = jit_module.make_signature();
    puts_sig.params.push(AbiParam::new(int64));
    puts_sig.returns.push(AbiParam::new(int64));

    // declare the puts function.
    // this is declared with Linkage::Import because it will be provided by the
    // system's dynamic linker
    let puts_func_id = jit_module
        .declare_function("puts", Linkage::Import, &puts_sig)
        .unwrap_or_else(|msg| panic!("Failed to declaring function: {}", msg));

    // declare the puts function in the function's scope
    let puts_func_ref = jit_module
        .declare_func_in_func(puts_func_id, &mut func_builder.func);

    // create a string as a global data object
    let hello_id = jit_module
        .declare_data("hello_world", Linkage::Local, false, false)
        .unwrap_or_else(|msg| panic!("Error while declaring data: {}", msg));

    let hello_data = b"Hello, Sailor!\0";

    let mut data_description = DataDescription::new();
    data_description.define(hello_data.to_vec().into_boxed_slice());

    jit_module.define_data(hello_id, &data_description)
        .unwrap_or_else(|msg| panic!("Error while defining data: {}", msg));

    // declare the string in the function's scope
    let hello_ptr = jit_module
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
    // transmute the function into a rust function
    
    let name = "main";

    let id = jit_module
        .declare_function(name, Linkage::Export, &codegen_context.func.signature)
        .unwrap_or_else(|e| { dbg!(e); panic!("failed to declare function") });

    jit_module
        .define_function(id, &mut codegen_context)
        .unwrap_or_else(|e| { dbg!(e); panic!("failed to define function") });

    //println!("---- Function: {} ----", name);
    //println!("{}", codegen_context.func.display());

    jit_module.clear_context(&mut codegen_context);
    jit_module.finalize_definitions()
        .unwrap_or_else(|e| { dbg!(e); panic!("failed to finalize definitions") });

    let code_ptr = jit_module.get_finalized_function(id);
    let code_fn = unsafe { std::mem::transmute::<_, fn(i64) -> i64>(code_ptr) };

    code_fn(0);
}

