use cranelift_frontend::FunctionBuilderContext;

use cranelift_module::DataDescription;
use cranelift_module::default_libcall_names;

use cranelift_object::ObjectModule;
use cranelift_object::ObjectBuilder;

fn main() {
    let flag_builder = cranelift_codegen::settings::builder();
    let flags = cranelift_codegen::settings::Flags::new(flag_builder);
    let isa = cranelift_native::builder()
        .unwrap_or_else(|msg| {
            panic!("error creating Cranelift native builder: {}", msg);
        })
        .finish(flags)
        .unwrap();

    let object_name = "CraneliftHello";
    let libcall_names = default_libcall_names();
    let object_builder = ObjectBuilder::new(isa, object_name, libcall_names)
        .expect("error creating object builder");

    let object_module = ObjectModule::new(object_builder);
    let func_context = FunctionBuilderContext::new();
    let data_description = DataDescription::new();
}
