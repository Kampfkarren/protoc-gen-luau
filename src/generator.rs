use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
};

use prost_types::{
    compiler::{
        code_generator_response::{Feature, File},
        CodeGeneratorRequest, CodeGeneratorResponse,
    },
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorProto,
};

use crate::{
    fields::{decode_field, wire_type_of_field_descriptor, FieldGenerator, FieldKind, WireType},
    if_builder::IfBuilder,
    string_builder::StringBuilder,
};

pub fn generate_response(request: CodeGeneratorRequest) -> CodeGeneratorResponse {
    let export_map = create_export_map(&request.proto_file);

    let mut files = Vec::new();

    files.push(File {
        name: Some("proto.luau".to_owned()),
        content: Some(include_str!("./luau/proto.luau").to_owned()),
        ..Default::default()
    });

    files.push(File {
        name: Some("base64.luau".to_owned()),
        content: Some(include_str!("./luau/base64.luau").to_owned()),
        ..Default::default()
    });

    files.append(
        &mut request
            .proto_file
            .into_iter()
            .filter_map(|file| {
                if file.syntax() != "proto3" {
                    eprintln!("Non-proto3 {} is not supported", file.name());
                    None
                } else {
                    Some(FileGenerator::new(file, &export_map).generate_file())
                }
            })
            .collect(),
    );

    CodeGeneratorResponse {
        error: None,
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
    let Some(options) = &descriptor.options else {
        return None;
    };

    if !options.map_entry() {
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

fn create_return(exports: Vec<String>) -> String {
    let mut lines = Vec::new();
    lines.push("return {".to_owned());
    for export in exports {
        lines.push(format!("\t{export} = {export},"));
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

const MESSAGE: &str = r#"<name> = {
    new = function()
        return {
<default>
        }
    end,

    encode = function(self: <name>): buffer
        local output = buffer.create(0)
        local cursor = 0

<encode>
        local shrunkBuffer = buffer.create(cursor)
        buffer.copy(shrunkBuffer, 0, output, 0, cursor)
        return shrunkBuffer
    end,

    decode = function(input: buffer): <name>
        local self = <name>.new()
        local cursor = 0

        while cursor < buffer.len(input) do
            local field, wireType
            field, wireType, cursor = proto.readTag(input, cursor)

            if wireType == proto.wireTypes.varint then
                local value
                value, cursor = proto.readVarInt(input, cursor)

                <decode_varint>
            elseif wireType == proto.wireTypes.lengthDelimited then
                local value
                value, cursor = proto.readBuffer(input, cursor)

                <decode_len>
            elseif wireType == proto.wireTypes.i32 then
                <decode_i32>
            elseif wireType == proto.wireTypes.i64 then
                <decode_i64>
            else
                error("Unsupported wire type: " .. wireType)
            end
        end

        return self
    end,

    jsonEncode = function(self: <name>): any
        local output: <json_type> = {}

        <json_encode>
        return output
    end,

    jsonDecode = function(input: { [string]: any }): <name>
        local self = <name>.new()

        <json_decode>
        return self
    end
}"#;

const ENUM: &str = r#"<name> = {
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
}

impl<'a> FileGenerator<'a> {
    fn new(
        file_descriptor_proto: FileDescriptorProto,
        export_map: &'a ExportMap,
    ) -> FileGenerator<'a> {
        Self {
            file_descriptor_proto,
            export_map,

            types: StringBuilder::new(),
            implementations: StringBuilder::new(),
            exports: Vec::new(),
        }
    }

    fn generate_file(mut self) -> File {
        let file_path = Path::new(self.file_descriptor_proto.name());

        let mut contents = StringBuilder::new();
        contents.push("--!strict");
        contents.push("--!nolint LocalUnused");
        contents.push(
            "--# selene: allow(empty_if, if_same_then_else, manual_table_clone, unused_variable)",
        );
        contents.push("-- This file was @autogenerated by protoc-gen-luau");

        // TODO: Reserve name
        contents.push(format!("local proto = require(\"{}proto\")", {
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

        for import in &self.file_descriptor_proto.dependency {
            let import_path = Path::new(import);

            let mut relative_import_path = pathdiff::diff_paths(
                import_path,
                file_path.parent().expect("couldn't get parent path"),
            )
            .expect("couldn't diff paths");

            if !relative_import_path.starts_with("../") {
                relative_import_path = PathBuf::from("./").join(relative_import_path);
            }

            contents.push(format!(
                "local {} = require(\"{}\")",
                file_path_export_name(import_path),
                relative_import_path.with_extension("").display()
            ));
        }

        contents.blank();

        for message in std::mem::take(&mut self.file_descriptor_proto.message_type) {
            self.generate_message(&message, "");
        }

        for descriptor in std::mem::take(&mut self.file_descriptor_proto.enum_type) {
            self.generate_enum(&descriptor, "");
        }

        contents.push(self.types.build());
        contents.push(self.implementations.build());
        contents.push(create_return(self.exports));

        let code = contents.build();

        File {
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
        }
    }

    fn generate_message(&mut self, message: &DescriptorProto, prefix: &str) {
        let name = format!("{prefix}{}", message.name());
        self.exports.push(name.clone());

        self.types
            .push(format!("local {name}: proto.Message<{name}>"));
        self.types.push(format!("export type {name} = {{"));
        self.types.indent();

        let mut json_type = StringBuilder::new();
        json_type.push("{");
        json_type.indent_n(3);

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
                    fields.push(FieldGenerator {
                        field_kind: FieldKind::OneOf {
                            name: oneof.to_owned(),
                            fields: vec![field],
                        },

                        export_map: self.export_map,
                        base_file: &self.file_descriptor_proto,
                    });
                }
            } else {
                fields.push(FieldGenerator {
                    field_kind: FieldKind::Single(field),
                    export_map: self.export_map,
                    base_file: &self.file_descriptor_proto,
                });
            }
        }

        // TODO: Make sure optional and required stuff makes sense between proto2/proto3
        for field in fields {
            self.types
                .push(format!("{}: {},", field.name(), field.type_definition()));

            json_type.push(format!("{}: {},", field.name(), field.json_type()));

            encode_lines.append(&mut field.encode());
            encode_lines.blank();

            json_encode_lines.append(&mut field.json_encode());
            json_encode_lines.blank();

            json_decode_lines.append(&mut field.json_decode());

            default_lines.push(format!("{} = {},", field.name(), field.default()));

            for inner_field in field.inner_fields() {
                let decoded = decode_field(
                    &format!("self.{}", field.name()),
                    inner_field,
                    self.export_map,
                    &self.file_descriptor_proto,
                    field.map_type(),
                    matches!(field.field_kind, FieldKind::OneOf { .. }),
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
            }
        }

        self.types.dedent();
        self.types.push("}");
        self.types.blank();

        json_type.dedent();
        json_type.push("}");

        self.implementations.push(
            MESSAGE
                .replace("    ", "\t")
                .replace("<name>", &name)
                .replace("<default>", &default_lines.build())
                .replace("<encode>", &encode_lines.build())
                .replace("<decode_varint>", &create_decoder(varint_fields))
                .replace("<decode_len>", &create_decoder(len_fields))
                .replace("<decode_i32>", &create_decoder(i32_fields))
                .replace("<decode_i64>", &create_decoder(i64_fields))
                .replace("<json_encode>", &json_encode_lines.build())
                .replace("<json_decode>", &json_decode_lines.build())
                .replace("<json_type>", &json_type.build()),
        );
        self.implementations.blank();

        for nested_message in &message.nested_type {
            if let Some(options) = &nested_message.options {
                if options.map_entry() {
                    continue;
                }
            }

            self.generate_message(nested_message, &format!("{name}_"));
        }

        for nested_enum in &message.enum_type {
            self.generate_enum(nested_enum, &format!("{name}_"));
        }
    }

    fn generate_enum(&mut self, descriptor: &EnumDescriptorProto, prefix: &str) {
        let name = format!("{prefix}{}", descriptor.name());

        self.types.push(format!("local {name}: proto.Enum<{name}>"));
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
}
