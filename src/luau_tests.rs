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
        "descriptors.proto",
        "enum_regression.proto",
        "forwards_compatibility.proto",
        "kitchen_sink.proto",
        "many_messages.proto",
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

    lune::Runtime::new()
        .unwrap()
        .run_custom(path.to_string_lossy(), contents)
        .expect("Error running test");
}

#[tokio::test]
async fn basic() {
    run_luau_test(Path::new("basic.luau")).await;
}

#[tokio::test]
async fn descriptors_require() {
    run_luau_test(Path::new("descriptors_require.luau")).await;
}

#[tokio::test]
async fn many_messages() {
    run_luau_test(Path::new("many_messages.luau")).await;
}

#[tokio::test]
async fn wkt_json() {
    run_luau_test(Path::new("wkt_json.luau")).await;
}

#[test]
fn descriptors_uses_it() {
    let file_descriptor_set = protox::Compiler::new(["./src/samples/protos"])
        .unwrap()
        .include_imports(true)
        .open_files(vec!["./src/samples/protos/descriptors_uses_it.proto"])
        .unwrap()
        .file_descriptor_set();

    assert!(
        crate::generator::generate_response(prost_types::compiler::CodeGeneratorRequest {
            file_to_generate: vec!["./src/samples/protos/descriptors_uses_it.proto".to_owned()],
            parameter: None,
            proto_file: file_descriptor_set.file,
            compiler_version: None,
        })
        .error
        .is_some()
    );
}
