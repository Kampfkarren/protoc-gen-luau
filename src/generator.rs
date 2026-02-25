use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use prost_types::{
    compiler::{
        code_generator_response::{Feature, File},
        CodeGeneratorRequest, CodeGeneratorResponse,
    },
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorProto,
};
use rayon::prelude::*;
use typed_path::{PathType, TypedPath, UnixPath as Path, UnixPathBuf as PathBuf};

use crate::{
    fields::{
        decode_field, decode_packed, is_packed, wire_type_of_field_descriptor, FieldGenerator,
        FieldKind, WireType,
    },
    if_builder::IfBuilder,
    string_builder::StringBuilder,
    wkt_json::WktJson,
};

// This is used for options in proto3, but is syntax = proto2.
// Don't import it, and error if we see it.
const DESCRIPTORS_IMPORT: &str = "google/protobuf/descriptor.proto";

pub fn generate_response(request: CodeGeneratorRequest) -> CodeGeneratorResponse {
    let export_map = Arc::new(create_export_map(&request.proto_file));

    let mut files = Vec::new();

    let options = request
        .parameter
        .map(|parameter| {
            parameter
                .split(',')
                .map(|option| option.splitn(2, '=').collect::<Vec<_>>())
                .filter(|option| option.len() == 2)
                .map(|option| (option[0].to_owned(), option[1].to_owned()))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();

    let roblox_imports = options.get("roblox_imports").map(|x| x.as_str()) == Some("true");

    let camel_case_fields = match options.get("field_name_case").map(|s| s.as_str()) {
        None => false,
        Some("snake") => false,
        Some("camel") => true,
        Some(invalid) => {
            return CodeGeneratorResponse {
                error: Some(format!(
                    "invalid field_name_case: \"{}\" (expected \"snake\" or \"camel\")",
                    invalid
                )),
                supported_features: Some(Feature::Proto3Optional as u64),
                file: Vec::new(),
            };
        }
    };

    let mut proto_init = include_str!("./luau/proto/init.luau").to_owned();
    if roblox_imports {
        proto_init = proto_init
            .replace("require(\"@self/base64\")", "require(script.base64)")
            .replace("require(\"@self/message\")", "require(script.message)")
            .replace(
                "require(\"@self/typeRegistry\")",
                "require(script.typeRegistry)",
            );
    }

    let mut type_registry_init = include_str!("./luau/proto/typeRegistry.luau").to_owned();
    if roblox_imports {
        type_registry_init =
            type_registry_init.replace("require(\"./message\")", "require(script.Parent.message)");
    }
    files.push(File {
        name: Some("proto/typeRegistry.luau".to_owned()),
        content: Some(type_registry_init),
        ..Default::default()
    });

    files.push(File {
        name: Some("proto/init.luau".to_owned()),
        content: Some(proto_init),
        ..Default::default()
    });

    files.push(File {
        name: Some("proto/message.luau".to_owned()),
        content: Some(include_str!("./luau/proto/message.luau").to_owned()),
        ..Default::default()
    });

    files.push(File {
        name: Some("proto/base64.luau".to_owned()),
        content: Some(include_str!("./luau/proto/base64.luau").to_owned()),
        ..Default::default()
    });

    // If we import the descriptor proto file, we need to explicitly block
    // everything it tries to import.
    // That way you can use descriptors for options, without needing to parse proto2.
    let mut forbidden_types = HashSet::new();

    let proto_files_to_process: Vec<_> = request
        .proto_file
        .into_iter()
        .filter(|file| {
            if file.name() == DESCRIPTORS_IMPORT {
                for enum_descriptor in &file.enum_type {
                    forbidden_types.insert(format!(".google.protobuf.{}", enum_descriptor.name()));
                }

                for message_descriptor in &file.message_type {
                    forbidden_types
                        .insert(format!(".google.protobuf.{}", message_descriptor.name()));

                    // Only goes one deep
                    for nested_type in &message_descriptor.nested_type {
                        forbidden_types.insert(format!(
                            ".google.protobuf.{}.{}",
                            message_descriptor.name(),
                            nested_type.name()
                        ));
                    }
                }
                false
            } else {
                true
            }
        })
        .collect();

    let forbidden_types = Arc::new(forbidden_types);

    let results: Vec<_> = proto_files_to_process
        .into_par_iter()
        .map(|file| {
            if file.syntax() != "proto3" {
                return Err(format!("{} is not proto3", file.name()));
            }

            let mut generator =
                FileGenerator::new(file, &export_map, &forbidden_types, camel_case_fields);

            if roblox_imports {
                generator.enable_roblox_imports();
            }

            let generated = generator.generate_file();
            Ok((generated.file, generated.errors))
        })
        .collect();

    let mut errors = Vec::new();
    let mut generated_files = Vec::new();

    for result in results {
        match result {
            Ok((file, file_errors)) => {
                generated_files.push(file);
                errors.extend(file_errors);
            }
            Err(error) => {
                errors.push(error);
            }
        }
    }

    files.extend(generated_files);

    CodeGeneratorResponse {
        error: if errors.is_empty() {
            None
        } else {
            Some(errors.join("\n"))
        },
        supported_features: Some(Feature::Proto3Optional as u64),
        file: files,
    }
}

#[derive(Debug)]
pub struct Export {
    pub path: PathBuf,
    pub prefix: String,
    pub map: Option<MapType>,
}

#[derive(Debug)]
pub struct MapType {
    pub key: FieldDescriptorProto,
    pub value: FieldDescriptorProto,
}

pub type ExportMap = HashMap<String, Export>;

fn create_export_map(files: &[FileDescriptorProto]) -> ExportMap {
    let mut export_map = HashMap::new();

    // todo: all descriptors
    for file in files {
        let path = PathBuf::from(file.name()).with_extension("");

        for descriptor in &file.enum_type {
            add_enum_descriptors(descriptor, &mut export_map, file.package(), &path, "");
        }

        for descriptor in &file.message_type {
            add_message_descriptors(descriptor, &mut export_map, file.package(), &path, "");
        }
    }

    export_map
}

fn add_message_descriptors(
    descriptor: &DescriptorProto,
    export_map: &mut ExportMap,
    package: &str,
    path: &Path,
    prefix: &str,
) {
    if export_map
        .insert(
            format!("{package}.{}", descriptor.name()),
            Export {
                path: path.to_path_buf(),
                prefix: prefix.to_owned(),
                map: extract_map(descriptor),
            },
        )
        .is_some()
    {
        unreachable!("duplicate message descriptor");
    }

    for nested_type in &descriptor.nested_type {
        add_message_descriptors(
            nested_type,
            export_map,
            &format!("{package}.{}", descriptor.name()),
            path,
            &format!("{prefix}{}_", descriptor.name()),
        );
    }

    for nested_enum in &descriptor.enum_type {
        add_enum_descriptors(
            nested_enum,
            export_map,
            &format!("{package}.{}", descriptor.name()),
            path,
            &format!("{prefix}{}_", descriptor.name()),
        );
    }
}

fn extract_map(descriptor: &DescriptorProto) -> Option<MapType> {
    if !descriptor.options.as_ref()?.map_entry() {
        return None;
    }

    let key = descriptor.field.iter().find(|field| field.number() == 1)?;
    let value = descriptor.field.iter().find(|field| field.number() == 2)?;

    Some(MapType {
        key: key.clone(),
        value: value.clone(),
    })
}

fn add_enum_descriptors(
    descriptor: &EnumDescriptorProto,
    export_map: &mut ExportMap,
    package: &str,
    path: &Path,
    prefix: &str,
) {
    if export_map
        .insert(
            format!("{package}.{}", descriptor.name()),
            Export {
                path: path.to_path_buf(),
                prefix: prefix.to_owned(),
                map: None,
            },
        )
        .is_some()
    {
        unreachable!("duplicate enum descriptor");
    }
}

fn create_messages_init(names: &[String]) -> String {
    let mut builder = StringBuilder::new();

    builder.blank();

    // We need to do this because referencing can happen out of order,
    // but at runtime it will always be safe.
    builder.push("type _Messages = {");
    builder.indent();

    for name in names {
        builder.push(format!("{name}: _{name}Message,"));
    }

    builder.dedent();
    builder.push("}");
    builder.push("local messages: _Messages = {} :: _Messages");

    builder.blank();

    builder.build()
}

fn create_return(exports: Vec<String>) -> String {
    let mut lines = Vec::new();
    lines.push("return {".to_owned());
    for export in exports {
        lines.push(format!("\t{export} = messages.{export},"));
    }
    lines.push("}\n".to_owned());
    lines.join("\n")
}

pub fn file_path_export_name(path: &Path) -> String {
    format!(
        "_{}",
        path.with_extension("").to_string_lossy().replace('/', "_")
    )
}

const MESSAGE: &str = r#"
do
    local _<name>Impl = {}
    _<name>Impl.__index = _<name>Impl

    function _<name>Impl.new(data: _<name>PartialFields?): <name>
        return setmetatable({
    <default>
        }, _<name>Impl :: _<name>Impl)
    end

    function _<name>Impl.encode(self: <name>): buffer
        local output = buffer.create(0)
        local cursor = 0

    <encode>
        local shrunkBuffer = buffer.create(cursor)
        buffer.copy(shrunkBuffer, 0, output, 0, cursor)
        return shrunkBuffer
    end

    function _<name>Impl.decode(input: buffer): <name>
        local self = _<name>Impl.new()
        local cursor = 0

        while cursor < buffer.len(input) do
            local field, wireType
            field, wireType, cursor = proto.readTag(input, cursor)

            if wireType == proto.wireTypes.varint then
                <decode_varint>

                local _
                _, cursor = proto.readVarInt(input, cursor)
            elseif wireType == proto.wireTypes.lengthDelimited then
                <decode_len>

                local length
                length, cursor = proto.readVarInt(input, cursor)

                cursor += length
            elseif wireType == proto.wireTypes.i32 then
                <decode_i32>

                local _
                _, cursor = proto.readFixed32(input, cursor)
            elseif wireType == proto.wireTypes.i64 then
                <decode_i64>

                local _
                _, cursor = proto.readFixed64(input, cursor)
            else
                error("Unsupported wire type: " .. wireType)
            end
        end

        return self
    end

    <json>

    _<name>Impl.descriptor = {
        name = "<name>",
        fullName = "<full_name>",
    }

    <any_methods>

    messages.<name> = _<name>Impl :: any  -- Luau: Not sure why this intersection fails.

    typeRegistry.default:register(messages.<name>)
end
"#;

const JSON: &str = r#"
function _<name>Impl.jsonEncode(self: <name>): any
    <json_encode>
end

function _<name>Impl.jsonDecode(input: { [string]: any }): <name>
    <json_decode>
end
"#;

const ENUM: &str = r#"messages.<name> = {
    fromNumber = function(value: number): <name>?
        <from_number>
    end,

    toNumber = function(self: <name>): number
        <to_number>
    end,

    fromName = function(name: string): <name>?
        <from_name>
    end,
}"#;

const ANY_METHOD_SIGNATURES: &str = r#"
-- Pack a message into an Any.
--
-- typeUrlPrefix should be the base URL for the type URL. For example, Google uses
-- "type.googleapis.com".
pack: (payload: proto.Message<any, any>, typeUrlPrefix: string) -> Any,

-- Returns the message contained by the Any (or nil if the Any is empty).
unpack: (self: Any, registry: typeRegistry.TypeRegistry?) -> proto.Message<any, any>?,

-- Returns true if and only if the Any contains an object of the type specified by
-- typeName. If typeName is a full type URL, it will be compared; otherwise,
-- only the type name will be compared.
isA: (self: Any, typeName: string) -> boolean,
"#;

fn create_decoder(fields: BTreeMap<i32, String>) -> String {
    if fields.is_empty() {
        return "-- No fields".to_owned();
    }

    let mut lines = StringBuilder::new();
    lines.indent_n(4);

    for (index, (field, code)) in fields.iter().enumerate() {
        lines.push(format!(
            "{} field == {field} then",
            if index == 0 { "if" } else { "elseif" }
        ));
        lines.push(format!("\t{code}"));
        lines.push("continue");
    }

    lines.push("end");
    lines.build().trim_start().to_owned()
}

struct FileGenerator<'a> {
    file_descriptor_proto: FileDescriptorProto,
    export_map: &'a ExportMap,

    types: StringBuilder,
    implementations: StringBuilder,

    exports: Vec<String>,
    names_defined_here: Vec<String>,
    errors: Vec<String>,

    forbidden_types: &'a HashSet<String>,

    roblox_imports: bool,
    camel_case_fields: bool,
}

struct FileAndErrors {
    file: File,
    errors: Vec<String>,
}

impl<'a> FileGenerator<'a> {
    fn new(
        file_descriptor_proto: FileDescriptorProto,
        export_map: &'a ExportMap,
        forbidden_types: &'a HashSet<String>,
        camel_case_fields: bool,
    ) -> FileGenerator<'a> {
        Self {
            file_descriptor_proto,
            export_map,

            types: StringBuilder::new(),
            implementations: StringBuilder::new(),
            names_defined_here: Vec::new(),
            exports: Vec::new(),
            errors: Vec::new(),

            forbidden_types,

            roblox_imports: false,
            camel_case_fields,
        }
    }

    fn enable_roblox_imports(&mut self) {
        self.roblox_imports = true;
    }

    fn generate_file(mut self) -> FileAndErrors {
        let file_path = Path::new(self.file_descriptor_proto.name());

        let mut contents = StringBuilder::new();
        contents.push("--!strict");
        contents.push("--!nolint LocalUnused");
        contents.push("--!nolint ImportUnused");
        contents.push(
            "--# selene: allow(empty_if, if_same_then_else, manual_table_clone, unused_variable)",
        );
        contents.push("-- This file was @autogenerated by protoc-gen-luau");

        let proto_require_path = PathBuf::from(format!("{}proto", {
            let mut path = file_path;
            let mut depth = 0;
            while path.parent().is_some() {
                path = path.parent().unwrap();
                depth += 1;
            }

            if depth == 1 {
                "./".to_owned()
            } else {
                "../".repeat(depth - 1)
            }
        }));

        // TODO: Reserve name
        contents.push(format!(
            "local proto = require({})",
            self.require_path(&proto_require_path)
        ));

        let mut type_registry_require_path = proto_require_path.clone();
        type_registry_require_path.push("typeRegistry");
        contents.push(format!(
            "local typeRegistry = require({})",
            self.require_path(&type_registry_require_path)
        ));

        // contents.push("\nlocal messages = {}");
        let line_to_insert_messages = contents.len();

        for import in &self.file_descriptor_proto.dependency {
            if import == DESCRIPTORS_IMPORT {
                continue;
            }

            let path_diff = pathdiff::diff_paths(
                std::path::Path::new(&import),
                std::path::Path::new(
                    &file_path
                        .parent()
                        .expect("couldn't get parent path")
                        .to_string_lossy()
                        .to_string(),
                ),
            )
            .expect("couldn't diff paths");

            let path_diff_str = path_diff.to_string_lossy().to_string();

            // TypedPath::derive() doesn't work with relative paths; it always considers them to be
            // Unix paths. So we need an explicit Windows check here.
            let path_type = if cfg!(windows) {
                PathType::Windows
            } else {
                PathType::Unix
            };
            let path_diff = TypedPath::new(&path_diff_str, path_type);
            let unix_path_diff = path_diff.with_unix_encoding();

            let mut relative_import_path =
                PathBuf::from(unix_path_diff.to_string_lossy().to_string());

            if !relative_import_path.starts_with("../") {
                relative_import_path = PathBuf::from("./").join(relative_import_path);
            }

            contents.push(format!(
                "local {} = require({})",
                file_path_export_name(Path::new(&import)),
                self.require_path(&relative_import_path.with_extension(""))
            ));
        }

        contents.blank();

        let package = self.file_descriptor_proto.package.clone();

        let scope = match package {
            Some(ref package) => package.as_str(),
            None => "",
        };

        for message in std::mem::take(&mut self.file_descriptor_proto.message_type) {
            self.generate_message(&message, "", scope);
        }

        for descriptor in std::mem::take(&mut self.file_descriptor_proto.enum_type) {
            self.generate_enum(&descriptor, "");
        }

        contents.push(self.types.build());
        contents.push(self.implementations.build());

        contents.insert(
            line_to_insert_messages,
            create_messages_init(&self.names_defined_here),
        );

        contents.push(create_return(self.exports));

        let code = contents.build();

        FileAndErrors {
            file: File {
                name: Some(
                    PathBuf::from(self.file_descriptor_proto.name())
                        .with_extension("luau")
                        .to_string_lossy()
                        .into_owned(),
                ),
                content: Some(
                    match stylua_lib::format_code(
                        &code,
                        stylua_lib::Config::default(),
                        None,
                        stylua_lib::OutputVerification::None,
                    ) {
                        Ok(formatted) => formatted,
                        Err(error) => {
                            eprintln!("Error formatting code: {error}");
                            code
                        }
                    },
                ),
                ..Default::default()
            },

            errors: self.errors,
        }
    }

    fn generate_message(&mut self, message: &DescriptorProto, prefix: &str, package: &str) {
        let name = format!("{prefix}{}", message.name());
        let full_name = format!("{package}.{}", message.name());

        if !message
            .options
            .as_ref()
            .map(|options| options.map_entry())
            .unwrap_or(false)
        {
            self.exports.push(name.clone());
        }

        self.names_defined_here.push(name.clone());

        let wkt_json = WktJson::try_create(&self.file_descriptor_proto, message);
        let json_type = match wkt_json.as_ref() {
            Some(wkt_json) => wkt_json.luau_type,
            None => "{ [string]: any }",
        };

        let is_wkt_any =
            self.file_descriptor_proto.package() == "google.protobuf" && message.name() == "Any";

        let maybe_any_method_signatures = if is_wkt_any {
            ANY_METHOD_SIGNATURES
        } else {
            ""
        };

        self.types.push(format!(
            r#"type _{name}Impl = {{
                __index: _{name}Impl,
                new: (fields: _{name}PartialFields?) -> {name},
                encode: (self: {name}) -> buffer,
                decode: (input: buffer) -> {name},
                jsonEncode: (self: {name}) -> {json_type},
                jsonDecode: (input: {json_type}) -> {name},
                descriptor: proto.Descriptor,
                {maybe_any_method_signatures}
            }}
            "#
        ));

        let mut fields_builder = StringBuilder::new();
        let mut partial_fields_builder = StringBuilder::new();

        fields_builder.push(format!(r#"type _{name}Fields = {{"#));
        fields_builder.indent();

        partial_fields_builder.push(format!(r#"type _{name}PartialFields = {{"#));
        partial_fields_builder.indent();

        let mut default_lines = StringBuilder::new();
        default_lines.indent_n(3);

        let mut encode_lines = StringBuilder::new();
        encode_lines.indent_n(2);

        let mut json_encode_lines = StringBuilder::new();
        json_encode_lines.indent_n(2);

        let mut json_decode_lines = StringBuilder::new();
        json_decode_lines.indent_n(2);

        let mut varint_fields: BTreeMap<i32, String> = BTreeMap::new();
        let mut len_fields: BTreeMap<i32, String> = BTreeMap::new();
        let mut i32_fields: BTreeMap<i32, String> = BTreeMap::new();
        let mut i64_fields: BTreeMap<i32, String> = BTreeMap::new();

        let mut fields: Vec<FieldGenerator<'_>> = Vec::new();
        for field in &message.field {
            if field.oneof_index.is_some() && !field.proto3_optional() {
                let oneof = message.oneof_decl[field.oneof_index.unwrap() as usize].name();

                if let Some(FieldKind::OneOf {
                    fields: ref mut existing_fields,
                    ..
                }) = fields
                    .iter_mut()
                    .find(|field| {
                        if let FieldKind::OneOf { name, .. } = &field.field_kind {
                            name == oneof
                        } else {
                            false
                        }
                    })
                    .map(|field| &mut field.field_kind)
                {
                    existing_fields.push(field);
                } else {
                    if self.forbidden_types.contains(field.type_name()) {
                        self.errors.push(format!(
                            "{}::{} is not supported",
                            message.name(),
                            field.name()
                        ));
                        continue;
                    }

                    fields.push(FieldGenerator {
                        field_kind: FieldKind::OneOf {
                            name: oneof.to_owned(),
                            fields: vec![field],
                        },

                        export_map: self.export_map,
                        base_file: &self.file_descriptor_proto,
                        camel_case_fields: self.camel_case_fields,
                    });
                }
            } else {
                if self.forbidden_types.contains(field.type_name()) {
                    self.errors.push(format!(
                        "{}::{} is not supported",
                        message.name(),
                        field.name()
                    ));
                    continue;
                }

                fields.push(FieldGenerator {
                    field_kind: FieldKind::Single(field),
                    export_map: self.export_map,
                    base_file: &self.file_descriptor_proto,
                    camel_case_fields: self.camel_case_fields,
                });
            }
        }

        // TODO: Make sure optional and required stuff makes sense between proto2/proto3
        for field in fields {
            let field_name = field.name();

            fields_builder.push(format!("{field_name}: {},", field.type_definition()));
            partial_fields_builder.push(format!(
                "{field_name}: {}?,",
                field.type_definition_no_presence()
            ));

            encode_lines.append(&mut field.encode());
            encode_lines.blank();

            if wkt_json.is_none() {
                json_encode_lines.append(&mut field.json_encode());
                json_encode_lines.blank();

                json_decode_lines.append(&mut field.json_decode());
            }

            default_lines.push(format!(
                r#"{field_name} = if data == nil or data.{field_name} == nil then {} else data.{field_name},"#,
                field.default()
            ));

            for inner_field in field.inner_fields() {
                let output = &format!("self.{field_name}");

                let decoded = decode_field(
                    output,
                    inner_field,
                    self.export_map,
                    &self.file_descriptor_proto,
                    field.map_type(),
                    matches!(field.field_kind, FieldKind::OneOf { .. }),
                    self.camel_case_fields,
                );

                match wire_type_of_field_descriptor(inner_field) {
                    WireType::Varint => {
                        varint_fields.insert(inner_field.number(), decoded.build());
                    }

                    WireType::LengthDelimited => {
                        len_fields.insert(inner_field.number(), decoded.build());
                    }

                    WireType::I32 => {
                        i32_fields.insert(inner_field.number(), decoded.build());
                    }

                    WireType::I64 => {
                        i64_fields.insert(inner_field.number(), decoded.build());
                    }
                }

                if is_packed(inner_field) {
                    len_fields.insert(inner_field.number(), decode_packed(inner_field, output));
                }
            }
        }

        fields_builder.dedent();
        fields_builder.push("}");
        fields_builder.blank();

        partial_fields_builder.dedent();
        partial_fields_builder.push("}");
        partial_fields_builder.blank();

        self.types.append(&mut fields_builder);
        self.types.blank();
        self.types.append(&mut partial_fields_builder);

        self.types.push(format!(
            "export type {name} = typeof(setmetatable({{}} :: _{name}Fields, {{}} :: _{name}Impl))"
        ));

        self.types.push(format!(
            "type _{name}Message = proto.Message<{name}, _{name}PartialFields> {}",
            if let Some(wkt_json) = wkt_json.as_ref() {
                format!(" & proto.CustomJson<{name}, {}>", wkt_json.luau_type)
            } else {
                String::new()
            }
        ));

        self.types.blank();

        let mut final_code = MESSAGE
            .replace("    ", "\t")
            .replace("<name>", &name)
            .replace("<full_name>", &full_name)
            .replace("<default>", &default_lines.build())
            .replace("<encode>", &encode_lines.build())
            .replace("<decode_varint>", &create_decoder(varint_fields))
            .replace("<decode_len>", &create_decoder(len_fields))
            .replace("<decode_i32>", &create_decoder(i32_fields))
            .replace("<decode_i64>", &create_decoder(i64_fields));

        if let Some(wkt_json) = WktJson::try_create(&self.file_descriptor_proto, message) {
            final_code = final_code.replace("<json>", &wkt_json.code);
        } else {
            final_code = final_code.replace(
                "<json>",
                &JSON
                    .replace("<name>", &name)
                    .replace(
                        "<json_encode>",
                        &format!(
                            "local output = {{}}\n\n{}\nreturn output",
                            &json_encode_lines.build()
                        ),
                    )
                    .replace(
                        "<json_decode>",
                        &format!(
                            "local self = _{name}Impl.new()\n\n{}\nreturn self",
                            &json_decode_lines.build()
                        ),
                    ),
            )
        }

        // Add special methods for google.protobuf.Any: pack, unpack, and isA.
        let any_methods = include_str!("./luau/wkt_mixins/Any_methods.luau");
        final_code = final_code.replace("<any_methods>", if is_wkt_any { any_methods } else { "" });

        self.implementations.push(final_code);
        self.implementations.blank();

        for nested_message in &message.nested_type {
            self.generate_message(nested_message, &format!("{name}_"), package);
        }

        for nested_enum in &message.enum_type {
            self.generate_enum(nested_enum, &format!("{name}_"));
        }
    }

    fn generate_enum(&mut self, descriptor: &EnumDescriptorProto, prefix: &str) {
        let name = format!("{prefix}{}", descriptor.name());

        self.types
            .push(format!("type _{name}Message = proto.Enum<{name}>"));
        self.types.push(format!("export type {name} ="));
        self.types.indent();

        let mut from_number = IfBuilder::new();
        from_number.indent_n(2);

        let mut enum_numbers_used = HashSet::new();

        let mut to_number = IfBuilder::new();
        to_number.indent_n(2);

        let mut from_name = IfBuilder::new();
        from_name.indent_n(2);

        for (index, field) in descriptor.value.iter().enumerate() {
            self.types.push(format!(
                "{}\"{}\"",
                if index == 0 { "" } else { "| " },
                field.name()
            ));

            if enum_numbers_used.insert(field.number()) {
                from_number.add_condition(&format!("value == {}", field.number()), |builder| {
                    builder.push(format!("return \"{}\"", field.name()));
                });
            }

            to_number.add_condition(&format!("self == \"{}\"", field.name()), |builder| {
                builder.push(format!("return {}", field.number()));
            });

            from_name.add_condition(&format!("name == \"{}\"", field.name()), |builder| {
                builder.push(format!("return \"{}\"", field.name()));
            });
        }

        self.types.push("| number -- Unknown");

        self.types.dedent();
        self.types.blank();

        self.exports.push(name.clone());
        self.names_defined_here.push(name.clone());

        self.implementations.push(
            ENUM.replace("<name>", &name)
                .replace(
                    "<from_number>",
                    from_number
                        .with_else(|builder| {
                            builder.push("return nil");
                        })
                        .build()
                        .trim_start(),
                )
                .replace(
                    "<to_number>",
                    to_number
                        .with_else(|builder| {
                            builder.push("return self");
                        })
                        .build()
                        .trim_start(),
                )
                .replace(
                    "<from_name>",
                    from_name
                        .with_else(|builder| {
                            builder.push("return nil");
                        })
                        .build()
                        .trim_start(),
                ),
        );
        self.implementations.blank();
    }

    fn require_path(&self, path: &Path) -> String {
        use typed_path::UnixComponent as Component;

        if self.roblox_imports {
            let mut pieces = Vec::new();
            pieces.push("script.Parent".to_owned());

            for component in path.components() {
                match component {
                    Component::CurDir => {}

                    Component::ParentDir => {
                        pieces.push("Parent".to_owned());
                    }

                    Component::Normal(name) => {
                        pieces.push(std::str::from_utf8(name).unwrap().to_string());
                    }

                    Component::RootDir => unreachable!(),
                }
            }

            pieces.join(".")
        } else {
            format!("\"{}\"", path.display())
        }
    }
}
