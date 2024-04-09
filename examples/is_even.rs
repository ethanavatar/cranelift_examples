use std::io::Write;

use cranelift_codegen::ir::AbiParam;
use cranelift_codegen::ir::InstBuilder;

use cranelift_frontend::FunctionBuilder;
use cranelift_frontend::FunctionBuilderContext;

use cranelift_jit::JITBuilder;
use cranelift_jit::JITModule;

use cranelift_module::Module;
use cranelift_module::Linkage;

fn main() {
    // ------------------------------------------------------------------------
    // general setup

    let flag_builder = cranelift_codegen::settings::builder();
    let flags = cranelift_codegen::settings::Flags::new(flag_builder);
    let isa = cranelift_native::builder()
        .unwrap_or_else(|msg| {
            panic!("Error while creating Cranelift native builder: {}", msg);
        })
        .finish(flags.clone())
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

    let mut builder = FunctionBuilder::new(
        &mut codegen_context.func,
        &mut func_builder_context
    );

    let entry_block = builder.create_block();
    builder.append_block_params_for_function_params(entry_block);
    builder.switch_to_block(entry_block);
    builder.seal_block(entry_block);

    let input_value = builder.block_params(entry_block)[0];
    let is_even = builder.ins().udiv_imm(input_value, 2);
    let is_even = builder.ins().imul_imm(is_even, 2);
    let is_even = builder.ins().isub(input_value, is_even);

    let then_block = builder.create_block();
    let else_block = builder.create_block();
    let after_block = builder.create_block();

    builder.append_block_param(after_block, int64);
    
    builder.ins().brif(
        is_even,
        then_block,
        &[],
        else_block,
        &[],
    );

    builder.switch_to_block(then_block);
    builder.seal_block(then_block);
    let const1 = builder.ins().iconst(int64, 1);
    builder.ins().jump(after_block, &[const1]);

    builder.switch_to_block(else_block);
    builder.seal_block(else_block);
    let const0 = builder.ins().iconst(int64, 0);
    builder.ins().jump(after_block, &[const0]);

    builder.switch_to_block(after_block);
    builder.seal_block(after_block);

    let result = builder.block_params(after_block)[0];
    builder.ins().return_(&[result]);

    builder.finalize();

    codegen_context.verify(&flags)
        .unwrap_or_else(|e| { dbg!(e); panic!("verification error") });

    // ------------------------------------------------------------------------
    // declare and define the main function
 
    let function_name = "main";
    let id = jit_module
        .declare_function(function_name, Linkage::Export, &codegen_context.func.signature)
        .unwrap_or_else(|e| { dbg!(e); panic!("failed to declare function") });

    jit_module
        .define_function(id, &mut codegen_context)
        .unwrap_or_else(|e| { dbg!(e); panic!("failed to define function") });

    // ------------------------------------------------------------------------
    // write the IR to a file

    let function_string = codegen_context.func.display().to_string();

    let out_dir = std::path::Path::new("out/is_even");

    if !out_dir.exists() {
        std::fs::create_dir_all(out_dir)
            .expect("error creating output directory");
    }

    let mut readable_ir = std::fs::File::create("out/is_even/is_even.clir").unwrap();
    readable_ir.write_all(function_string.as_bytes())
        .expect("error writing IR to file");

    // ------------------------------------------------------------------------
    // compile the function

    jit_module.clear_context(&mut codegen_context);
    jit_module.finalize_definitions()
        .unwrap_or_else(|e| { dbg!(e); panic!("failed to finalize definitions") });

    // ------------------------------------------------------------------------
    // transmute the function into a rust function and call it

    let code_ptr = jit_module.get_finalized_function(id);
    let code_fn = unsafe {
        std::mem::transmute::<_, fn(i64) -> i64>(code_ptr)
    };

    let program_args = std::env::args().collect::<Vec<String>>();
    let input = program_args[1].parse::<i64>().unwrap();

    // call the function
    match code_fn(input) {
        0 => println!("{} is even", input),
        1 => println!("{} is odd", input),
        _ => unreachable!(),
    }
}

