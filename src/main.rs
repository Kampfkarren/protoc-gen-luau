#[allow(unused_imports)] // todo: remove
use std::io::{Read, Write};

use color_eyre::eyre::WrapErr;
use prost::Message;

mod generator;
mod if_builder;
mod string_builder;

fn main() -> color_eyre::Result<()> {
    let stdin = std::io::stdin();
    let mut bytes: Vec<u8> = Vec::new();
    stdin
        .lock()
        .read_to_end(&mut bytes)
        .wrap_err("couldn't read to end of stdin")?;

    let mut output = Vec::new();
    generator::generate_response(prost_types::compiler::CodeGeneratorRequest::decode(bytes.as_slice()).wrap_err(
        "couldn't parse CodeGeneratorRequest, make sure you're using this as a plugin to protoc",
    )?).encode(&mut output).wrap_err("couldn't encode CodeGeneratorResponse")?;

    std::io::stdout()
        .write_all(&output)
        .wrap_err("couldn't write to stdout")?;

    Ok(())
}
