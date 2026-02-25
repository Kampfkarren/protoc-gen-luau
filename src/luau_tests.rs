use std::path::Path;

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
        .await
        .expect("Error running test");
}

/// Extracts the body of `type _{message_name}Fields = { ... }` so tests can assert on
/// field names in the type definition only, not elsewhere in the file (e.g. decode branches).
fn extract_fields_type_block<'a>(content: &'a str, message_name: &str) -> &'a str {
    let prefix = format!("type _{message_name}Fields = {{");
    let start = content
        .find(&prefix)
        .unwrap_or_else(|| panic!("missing type _{message_name}Fields in generated content"));
    let body_start = start + prefix.len();
    let mut depth = 1u32;
    let mut i = body_start;
    let bytes = content.as_bytes();
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        i += 1;
    }
    &content[body_start..i.saturating_sub(1)]
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

#[test]
fn field_name_case_invalid_returns_error() {
    let file_descriptor_set = protox::Compiler::new(["./src/samples/protos"])
        .unwrap()
        .include_imports(true)
        .open_files(vec!["./src/samples/protos/field_case_test.proto"])
        .unwrap()
        .file_descriptor_set();

    let response = crate::generator::generate_response(prost_types::compiler::CodeGeneratorRequest {
        file_to_generate: vec!["./src/samples/protos/field_case_test.proto".to_owned()],
        parameter: Some("field_name_case=other".to_owned()),
        proto_file: file_descriptor_set.file,
        compiler_version: None,
    });

    assert!(response.error.is_some(), "expected error for invalid field_name_case");
    assert!(
        response.error.as_deref().unwrap().contains("invalid field_name_case"),
        "error message should mention invalid field_name_case"
    );
    assert!(
        response.file.is_empty(),
        "should not generate any files when option is invalid"
    );
}

#[test]
fn field_name_case_default_generates_snake_case_fields() {
    let file_descriptor_set = protox::Compiler::new(["./src/samples/protos"])
        .unwrap()
        .include_imports(true)
        .open_files(vec!["./src/samples/protos/field_case_test.proto"])
        .unwrap()
        .file_descriptor_set();

    let response = crate::generator::generate_response(prost_types::compiler::CodeGeneratorRequest {
        file_to_generate: vec!["./src/samples/protos/field_case_test.proto".to_owned()],
        parameter: None,
        proto_file: file_descriptor_set.file,
        compiler_version: None,
    });

    assert!(response.error.is_none(), "generation should succeed: {:?}", response.error);
    let content: &str = response
        .file
        .iter()
        .find(|f| f.name().contains("field_case_test"))
        .map(|f| f.content())
        .expect("should generate field_case_test Luau file");
    let fields_type = extract_fields_type_block(content, "FieldCaseTest");
    assert!(
        fields_type.contains("string_value:") && fields_type.contains("other_field:"),
        "default (no option) should produce snake_case in _FieldCaseTestFields; got: {fields_type:?}"
    );
}

#[test]
fn field_name_case_snake_generates_snake_case_fields() {
    let file_descriptor_set = protox::Compiler::new(["./src/samples/protos"])
        .unwrap()
        .include_imports(true)
        .open_files(vec!["./src/samples/protos/field_case_test.proto"])
        .unwrap()
        .file_descriptor_set();

    let response = crate::generator::generate_response(prost_types::compiler::CodeGeneratorRequest {
        file_to_generate: vec!["./src/samples/protos/field_case_test.proto".to_owned()],
        parameter: Some("field_name_case=snake".to_owned()),
        proto_file: file_descriptor_set.file,
        compiler_version: None,
    });

    assert!(response.error.is_none(), "generation should succeed: {:?}", response.error);
    let content: &str = response
        .file
        .iter()
        .find(|f| f.name().contains("field_case_test"))
        .map(|f| f.content())
        .expect("should generate field_case_test Luau file");
    let fields_type = extract_fields_type_block(content, "FieldCaseTest");
    assert!(
        fields_type.contains("string_value:") && fields_type.contains("other_field:"),
        "snake option should produce snake_case in _FieldCaseTestFields; got: {fields_type:?}"
    );
}

#[test]
fn field_name_case_camel_generates_camel_case_fields() {
    let file_descriptor_set = protox::Compiler::new(["./src/samples/protos"])
        .unwrap()
        .include_imports(true)
        .open_files(vec!["./src/samples/protos/field_case_test.proto"])
        .unwrap()
        .file_descriptor_set();

    let response = crate::generator::generate_response(prost_types::compiler::CodeGeneratorRequest {
        file_to_generate: vec!["./src/samples/protos/field_case_test.proto".to_owned()],
        parameter: Some("field_name_case=camel".to_owned()),
        proto_file: file_descriptor_set.file,
        compiler_version: None,
    });

    assert!(response.error.is_none(), "generation should succeed: {:?}", response.error);
    let content: &str = response
        .file
        .iter()
        .find(|f| f.name().contains("field_case_test"))
        .map(|f| f.content())
        .expect("should generate field_case_test Luau file");
    let fields_type = extract_fields_type_block(content, "FieldCaseTest");
    assert!(
        fields_type.contains("stringValue:") && fields_type.contains("otherField:"),
        "camel option should produce camelCase in _FieldCaseTestFields; got: {fields_type:?}"
    );
}
