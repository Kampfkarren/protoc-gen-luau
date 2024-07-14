use std::{path::Path, process::ExitCode};

use tokio::sync::OnceCell;

async fn create_samples_once() {
    static ONCE: OnceCell<()> = OnceCell::const_new();

    ONCE.get_or_init(|| async {
        generate_samples();
    })
    .await;
}

fn generate_samples() {
    let files = [
        "enum_regression.proto",
        "forwards_compatibility.proto",
        "kitchen_sink.proto",
        "recursive.proto",
        "wkt.proto",
    ];

    let file_descriptor_set = protox::Compiler::new(["./src/samples/protos"])
        .unwrap()
        .include_imports(true)
        .open_files(files)
        .unwrap()
        .file_descriptor_set();

    let response =
        crate::generator::generate_response(prost_types::compiler::CodeGeneratorRequest {
            file_to_generate: files.iter().map(|&string| string.to_owned()).collect(),
            parameter: None,
            proto_file: file_descriptor_set.file,
            compiler_version: None,
        });

    assert_eq!(response.error, None);

    let samples_directory = Path::new("src/tests/samples");

    std::fs::remove_dir_all(samples_directory).ok();
    std::fs::create_dir(samples_directory).unwrap();

    for proto_file in response.file {
        let path = samples_directory.join(Path::new(proto_file.name()));
        std::fs::create_dir_all(path.parent().unwrap()).ok();

        std::fs::write(path, proto_file.content()).unwrap();
    }
}

async fn run_luau_test(filename: &Path) {
    create_samples_once().await;

    let path = Path::new("src/tests/").join(filename);
    let contents = std::fs::read_to_string(&path).unwrap();

    let exit_code = lune::Runtime::new()
        .run(path.to_string_lossy(), contents)
        .await
        .expect("Error running test");

    // HACK: https://github.com/lune-org/lune/issues/175 must be fixed
    if format!("{exit_code:?}") == format!("{:?}", ExitCode::FAILURE) {
        panic!("Test failed. You may need to run again with -- --nocapture to see the output if you haven't already.");
    }
}

#[tokio::test]
async fn basic() {
    run_luau_test(Path::new("basic.luau")).await;
}

#[tokio::test]
async fn wkt_json() {
    run_luau_test(Path::new("wkt_json.luau")).await;
}
