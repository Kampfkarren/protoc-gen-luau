use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, Mutex, OnceLock,
    },
};

fn wait_for_samples() {
    static SAMPLES_GENERATED: OnceLock<Arc<(Mutex<bool>, Condvar)>> = OnceLock::new();
    static TRIED_GENERATING: AtomicBool = AtomicBool::new(false);

    let (lock, cvar) = &*Arc::clone(
        SAMPLES_GENERATED.get_or_init(|| Arc::new((Mutex::new(false), Condvar::new()))),
    );

    if !TRIED_GENERATING.swap(true, Ordering::SeqCst) {
        generate_samples();

        let mut generated = lock.lock().unwrap();
        *generated = true;
    }

    let mut generated = lock.lock().unwrap();
    while !*generated {
        generated = cvar.wait(generated).unwrap();
    }
}

fn generate_samples() {
    let files = ["kitchen_sink.proto", "wkt.proto"];

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
    wait_for_samples();

    let path = Path::new("src/tests/").join(filename);
    let contents = std::fs::read_to_string(&path).unwrap();

    lune::Runtime::new()
        .run(path.to_string_lossy(), contents)
        .await
        .expect("Error running test");
}

#[tokio::test]
async fn basic() {
    run_luau_test(Path::new("basic.luau")).await;
    run_luau_test(Path::new("wkt_json.luau")).await;
}
