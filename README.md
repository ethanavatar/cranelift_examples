# Cranelift Examples

A collection of examples using the [Cranelift](https://github.com/bytecodealliance/wasmtime/tree/main/cranelift) code generator.

I will try to keep these examples updated as I learn how to use Cranelift myself. Right now, I'm just getting started.

## Examples
- [hello_object](#hello_object)
- [hello_jit](#hello_jit)

## hello_object

This example uses [cranelift_object](https://docs.rs/cranelift-object/latest/cranelift_object/) to generate a native object file for a hello world program.

```sh
$ cargo run --example hello_object
```

This will generate a `hello.o` file in the `out` directory and compile it with `clang` into `a.exe`.
A `hello.clir` file will also be genrated in the `out` directory containing a readable representation of the Cranelift IR that was generated.

## hello_jit

This example uses [cranelift_jit](https://docs.rs/cranelift-jit/latest/cranelift_jit/) to JIT compile a hello world program into a function that can be used at runtime.

```sh
$ cargo run --example hello_jit
Hello, Sailor!
```

A `hello.clir` file will also be genrated in the `out` directory containing a readable representation of the Cranelift IR that was generated.
