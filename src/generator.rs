use std::{
    borrow::Cow,
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};

use prost_types::{
    compiler::{
        code_generator_response::{Feature, File},
        CodeGeneratorRequest, CodeGeneratorResponse,
    },
    field_descriptor_proto::{Label, Type},
    DescriptorProto, EnumDescriptorProto, FieldDescriptorProto, FileDescriptorProto,
};

use crate::{if_builder::IfBuilder, string_builder::StringBuilder};

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
struct Export {
    path: PathBuf,
    prefix: String,
}
type ExportMap = HashMap<String, Export>;

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

fn file_path_export_name(path: &Path) -> String {
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

    jsonDecode = function(input: any): <name>
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
    end
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

// todo: move all this to a separate file
// todo: put export_map and base_file as fields in a struct
#[derive(Debug)]
enum Field<'a> {
    Normal(&'a FieldDescriptorProto),
    OneOf {
        name: String,
        fields: Vec<&'a FieldDescriptorProto>,
    },
}

impl Field<'_> {
    // In a simple sense: will this be T? or T
    fn has_presence(&self) -> bool {
        match self {
            Field::Normal(field) => {
                (field.label.is_some() && field.label() == Label::Optional)
                    || matches!(field.r#type(), Type::Message)
            }

            Field::OneOf { .. } => true,
        }
    }

    fn name(&self) -> String {
        match self {
            Field::Normal(field) => field.name().to_owned(),
            Field::OneOf { name, .. } => name.to_owned(),
        }
    }

    fn type_definition_no_presence(
        &self,
        export_map: &ExportMap,
        base_file: &FileDescriptorProto,
    ) -> String {
        match self {
            Field::Normal(field) => format!("{}: {}", field.name(), {
                let definition = type_definition_of_field_descriptor(field, export_map, base_file);

                if field.label.is_some() && field.label() == Label::Repeated {
                    format!("{{ {definition} }}")
                } else {
                    definition
                }
            }),

            Field::OneOf { name, fields } => {
                let variants = fields
                    .iter()
                    .map(|field| {
                        format!(
                            "{{ type: \"{}\", value: {} }}",
                            field.name(),
                            type_definition_of_field_descriptor(field, export_map, base_file)
                        )
                    })
                    .collect::<Vec<_>>();

                format!("{}: ({})", name, variants.join(" | "))
            }
        }
    }

    fn type_definition(&self, export_map: &ExportMap, base_file: &FileDescriptorProto) -> String {
        let mut definition = self.type_definition_no_presence(export_map, base_file);

        if self.has_presence() {
            definition.push('?');
        }

        definition
    }

    fn json_type(&self, export_map: &ExportMap, base_file: &FileDescriptorProto) -> String {
        if let Field::OneOf { name, fields, .. } = self {
            let variants = fields
                .iter()
                .map(|field| type_definition_of_field_descriptor(field, export_map, base_file))
                .collect::<Vec<_>>();

            return format!("{name}: ({})?", variants.join(" | "));
        }

        let mut definition = self.type_definition_no_presence(export_map, base_file);
        definition.push('?');
        definition
    }

    fn should_encode(&self, export_map: &ExportMap, base_file: &FileDescriptorProto) -> String {
        let this = format!("self.{}", self.name());

        if self.has_presence() {
            return format!("{this} ~= nil");
        }

        let Field::Normal(field) = self else {
            unreachable!("OneOf has presence");
        };

        if field.label.is_some() && field.label() == Label::Repeated {
            return format!("#{this} > 0");
        }

        match field.r#type() {
            Type::Int32 | Type::Uint32 | Type::Float | Type::Double => format!("{this} ~= 0"),
            Type::String => format!("{this} ~= \"\""),
            Type::Bool => this,
            Type::Bytes => format!("buffer.len({this}) > 0"),
            Type::Enum => format!(
                "{this} ~= 0 or {this} ~= {}.fromNumber(0)",
                type_definition_of_field_descriptor(field, export_map, base_file)
            ),
            Type::Message => unreachable!("Message has presence"),
            other => unimplemented!("Unsupported type: {other:?}"),
        }
    }

    fn encode(&self, export_map: &ExportMap, base_file: &FileDescriptorProto) -> StringBuilder {
        let this = format!("self.{}", self.name());

        let mut encode = StringBuilder::new();
        encode.push(format!(
            "if {} then",
            self.should_encode(export_map, base_file)
        ));

        match self {
            Field::Normal(field) => {
                if field.label.is_some() && field.label() == Label::Repeated {
                    encode.push(format!("for _, value in {this} do"));
                    encode.indent();

                    encode.push(encode_field_descriptor_ignore_repeated(
                        field, export_map, base_file, "value",
                    ));

                    encode.dedent();
                    encode.push("end");
                } else {
                    encode.push(encode_field_descriptor_ignore_repeated(
                        field, export_map, base_file, &this,
                    ));
                }
            }

            Field::OneOf { fields, .. } => {
                let mut if_builder = IfBuilder::new();

                for field in fields {
                    if_builder.add_condition(
                        &format!("{this}.type == \"{}\"", field.name()),
                        |builder| {
                            builder.push(encode_field_descriptor_ignore_repeated(
                                field,
                                export_map,
                                base_file,
                                &format!("{this}.value"),
                            ));
                        },
                    );
                }

                encode.append(&mut if_builder.into_string_builder())
            }
        }

        encode.push("end");
        encode
    }

    fn json_encode(
        &self,
        export_map: &ExportMap,
        base_file: &FileDescriptorProto,
    ) -> StringBuilder {
        let this = format!("self.{}", self.name());
        let output = format!("output.{}", self.name());

        let mut json_encode = StringBuilder::new();
        json_encode.push(format!(
            "if {} then",
            self.should_encode(export_map, base_file)
        ));

        match self {
            Field::Normal(field) => {
                if field.label.is_some() && field.label() == Label::Repeated {
                    json_encode.push("local newOutput = {}");
                    json_encode.push(format!("for _, value in {this} do"));
                    json_encode.push(format!(
                        "table.insert(newOutput, {})",
                        json_encode_instruction_field_descriptor_ignore_repeated(
                            field, export_map, base_file, "value"
                        )
                    ));
                    json_encode.push("end");
                    json_encode.push(format!("{output} = newOutput"));
                } else {
                    json_encode.push(format!(
                        "{output} = {}",
                        json_encode_instruction_field_descriptor_ignore_repeated(
                            field, export_map, base_file, &this
                        )
                    ));
                }
            }

            Field::OneOf { fields, .. } => {
                let mut if_builder = IfBuilder::new();

                for field in fields {
                    if_builder.add_condition(
                        &format!("{this}.type == \"{}\"", field.name()),
                        |builder| {
                            builder.push(format!(
                                "{output} = {}",
                                json_encode_instruction_field_descriptor_ignore_repeated(
                                    field,
                                    export_map,
                                    base_file,
                                    &format!("{this}.value")
                                )
                            ));
                        },
                    );
                }

                json_encode.append(&mut if_builder.into_string_builder())
            }
        }

        json_encode.push("end");
        json_encode
    }

    fn inner_fields(&self) -> Vec<&FieldDescriptorProto> {
        match self {
            Field::Normal(field) => vec![field],
            Field::OneOf { fields, .. } => fields.clone(),
        }
    }

    fn default(
        &self,
        export_map: &ExportMap,
        base_file: &FileDescriptorProto,
    ) -> Cow<'static, str> {
        match self {
            Field::Normal(field) => {
                if field.label.is_some() && field.label() == Label::Repeated {
                    return "{}".into();
                }

                match field.r#type() {
                    Type::Int32 | Type::Uint32 | Type::Float | Type::Double => "0".into(),
                    Type::String => "\"\"".into(),
                    Type::Bool => "false".into(),
                    Type::Bytes => "buffer.create(0)".into(),
                    Type::Enum => format!(
                        "{}.fromNumber(0)",
                        type_definition_of_field_descriptor(field, export_map, base_file)
                    )
                    .into(),
                    Type::Message => format!(
                        "{}.new()",
                        type_definition_of_field_descriptor(field, export_map, base_file)
                    )
                    .into(),
                    other => unimplemented!("Unsupported type: {other:?}"),
                }
            }

            Field::OneOf { .. } => "nil".into(),
        }
    }
}

fn type_definition_of_field_descriptor(
    field: &FieldDescriptorProto,
    export_map: &ExportMap,
    base_file: &FileDescriptorProto,
) -> String {
    match field.r#type() {
        Type::Int32 | Type::Uint32 => "number".to_owned(),
        Type::Float => "number".to_owned(),
        Type::Double => "number".to_owned(),
        Type::String => "string".to_owned(),
        Type::Bool => "boolean".to_owned(),
        Type::Bytes => "buffer".to_owned(),
        Type::Enum | Type::Message => {
            let type_name = field.type_name();
            assert!(
                type_name.starts_with('.'),
                "NYI: Relative type names: {type_name:?}"
            );

            let type_name = &type_name[1..];

            let mut segments: Vec<&str> = type_name.split('.').collect();
            let just_type = segments.pop().unwrap();
            let package = segments.join(".");

            if package == base_file.package() {
                just_type.to_owned()
            } else {
                let export = export_map
                    .get(&format!("{package}.{just_type}"))
                    .unwrap_or_else(|| panic!("couldn't find export {package}.{just_type}"));

                if export.path == Path::new(base_file.name()).with_extension("") {
                    format!("{}{just_type}", export.prefix)
                } else {
                    format!(
                        "{}.{}{just_type}",
                        file_path_export_name(&export.path),
                        export.prefix,
                    )
                }
            }
        }
        other => unimplemented!("Unsupported type: {other:?}"),
    }
}

enum WireType {
    Varint,
    LengthDelimited,
    I32,
    I64,
}

fn wire_type_of_field_descriptor(field: &FieldDescriptorProto) -> WireType {
    match field.r#type() {
        Type::Int32 | Type::Uint32 | Type::Enum | Type::Bool => WireType::Varint,
        Type::Float => WireType::I32,
        Type::Double => WireType::I64,
        Type::String | Type::Bytes | Type::Message => WireType::LengthDelimited,
        other => unimplemented!("Unsupported type: {other:?}"),
    }
}

fn encode_field_descriptor_ignore_repeated(
    field: &FieldDescriptorProto,
    export_map: &ExportMap,
    base_file: &FileDescriptorProto,
    value_var: &str,
) -> String {
    match field.r#type() {
        Type::Int32 | Type::Uint32 => [
            format!(
                "output, cursor = proto.writeTag(output, cursor, {}, proto.wireTypes.varint)",
                field.number()
            ),
            format!("output, cursor = proto.writeVarInt(output, cursor, {value_var})"),
        ]
        .join("\n"),

        Type::Float => [
            format!(
                "output, cursor = proto.writeTag(output, cursor, {}, proto.wireTypes.i32)",
                field.number()
            ),
            format!("output, cursor = proto.writeFloat(output, cursor, {value_var})"),
        ]
        .join("\n"),

        Type::Double => [
            format!(
                "output, cursor = proto.writeTag(output, cursor, {}, proto.wireTypes.i64)",
                field.number()
            ),
            format!("output, cursor = proto.writeDouble(output, cursor, {value_var})"),
        ]
        .join("\n"),

        Type::String => {
            [
                format!(
                    "output, cursor = proto.writeTag(output, cursor, {}, proto.wireTypes.lengthDelimited)",
                    field.number()
                ),
                format!("output, cursor = proto.writeString(output, cursor, {value_var})"),
            ]
            .join("\n")
        }

        Type::Bool => {
            [
                format!(
                    "output, cursor = proto.writeTag(output, cursor, {}, proto.wireTypes.varint)",
                    field.number()
                ),
                format!(
                    "output, cursor = proto.writeVarInt(output, cursor, if {value_var} then 1 else 0)",
                ),
            ]
            .join("\n")
        }

        Type::Bytes => {
            [
                format!(
                    "output, cursor = proto.writeTag(output, cursor, {}, proto.wireTypes.lengthDelimited)",
                    field.number()
                ),
                format!("output, cursor = proto.writeBuffer(output, cursor, {value_var})"),
            ]
            .join("\n")
        }

        Type::Enum => {
            [
                format!(
                    "output, cursor = proto.writeTag(output, cursor, {}, proto.wireTypes.varint)",
                    field.number()
                ),
                format!(
                    "output, cursor = proto.writeVarInt(output, cursor, {}.toNumber({value_var}))",
                    type_definition_of_field_descriptor(field, export_map, base_file)
                ),
            ]
            .join("\n")
        }

        Type::Message => {
            [
                format!(
                    "local encoded = {}.encode({value_var})",
                    type_definition_of_field_descriptor(field, export_map, base_file)
                ),
                format!(
                    "output, cursor = proto.writeTag(output, cursor, {}, proto.wireTypes.lengthDelimited)",
                    field.number()
                ),
                format!(
                    "output, cursor = proto.writeVarInt(output, cursor, buffer.len(encoded))",
                ),
                format!("output, cursor = proto.writeBuffer(output, cursor, encoded)"),
            ]
            .join("\n")
        }

        other => unimplemented!("Unsupported type: {other:?}"),
    }
}

fn json_encode_instruction_field_descriptor_ignore_repeated(
    field: &FieldDescriptorProto,
    export_map: &ExportMap,
    base_file: &FileDescriptorProto,
    value_var: &str,
) -> String {
    match field.r#type() {
        Type::Int32 | Type::Int64 | Type::Bool | Type::String => value_var.to_owned(),
        Type::Float | Type::Double => format!("proto.json.serializeNumber({value_var})"),
        Type::Bytes => "proto.json.serializeBuffer({value_var})".to_owned(),
        Type::Enum => format!(
            "if typeof({value_var}) == \"number\" then {value_var} else {}.toNumber({value_var})",
            type_definition_of_field_descriptor(field, export_map, base_file)
        ),
        Type::Message => format!(
            "{}.jsonEncode({value_var})",
            type_definition_of_field_descriptor(field, export_map, base_file)
        ),
        other => unimplemented!("Unsupported type: {other:?}"),
    }
}

fn decode_instruction_field_descriptor_ignore_repeated(
    field: &FieldDescriptorProto,
    export_map: &ExportMap,
    base_file: &FileDescriptorProto,
) -> Cow<'static, str> {
    match field.r#type() {
        Type::Int32 | Type::Uint32 | Type::Float | Type::Double | Type::Bytes => "value".into(),

        Type::Bool => "value ~= 0".into(),

        Type::String => "buffer.tostring(value)".into(),

        Type::Enum => format!(
            "{}.fromNumber(value) or value",
            type_definition_of_field_descriptor(field, export_map, base_file)
        )
        .into(),

        Type::Message => format!(
            "{}.decode(value)",
            type_definition_of_field_descriptor(field, export_map, base_file)
        )
        .into(),

        other => unimplemented!("Unsupported type: {other:?}"),
    }
}

fn decode_field(
    this: &str,
    field: &FieldDescriptorProto,
    export_map: &ExportMap,
    base_file: &FileDescriptorProto,
    is_oneof: bool,
) -> StringBuilder {
    let mut decode = StringBuilder::new();

    if field.label.is_some() && field.label() == Label::Repeated {
        decode.push(format!("if {this} == nil then"));
        decode.indent();
        decode.push(format!("{this} = {{}}"));
        decode.dedent();
        decode.push("end");
        decode.push(format!("assert({this} ~= nil, \"Luau\")"));
        decode.blank();

        decode.push(format!(
            "table.insert({this}, {})",
            decode_instruction_field_descriptor_ignore_repeated(field, export_map, base_file)
        ));
    } else {
        match field.r#type() {
            Type::Float => {
                decode.push("local value");
                decode.push("value, cursor = proto.readFloat(input, cursor)");
            }

            Type::Double => {
                decode.push("local value");
                decode.push("value, cursor = proto.readDouble(input, cursor)");
            }

            _ => {}
        }

        if is_oneof {
            decode.push(format!(
                "{this} = {{ type = \"{}\", value = {} }}",
                field.name(),
                decode_instruction_field_descriptor_ignore_repeated(field, export_map, base_file)
            ));
        } else {
            decode.push(format!(
                "{this} = {}",
                decode_instruction_field_descriptor_ignore_repeated(field, export_map, base_file)
            ));
        }
    }

    decode
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
        contents.push("--# selene: allow(empty_if, if_same_then_else, unused_variable)");
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

            let parent_path = pathdiff::diff_paths(
                file_path.parent().unwrap_or_else(|| Path::new("")),
                import_path.parent().unwrap_or_else(|| Path::new("")),
            )
            .expect("couldn't diff paths");

            contents.push(format!(
                "local {} = require(\"{}/{}\")",
                file_path_export_name(import_path),
                if parent_path.as_os_str().is_empty() {
                    ".".to_owned()
                } else {
                    parent_path.to_string_lossy().into_owned()
                },
                import_path.with_extension("").display()
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

        let mut varint_fields: BTreeMap<i32, String> = BTreeMap::new();
        let mut len_fields: BTreeMap<i32, String> = BTreeMap::new();
        let mut i32_fields: BTreeMap<i32, String> = BTreeMap::new();
        let mut i64_fields: BTreeMap<i32, String> = BTreeMap::new();

        let mut fields = Vec::new();
        for field in &message.field {
            if field.oneof_index.is_some() && !field.proto3_optional() {
                let oneof = message.oneof_decl[field.oneof_index.unwrap() as usize].name();

                if let Some(Field::OneOf {
                    fields: ref mut existing_fields,
                    ..
                }) = fields.iter_mut().find(|field| {
                    if let Field::OneOf { name, .. } = field {
                        name == oneof
                    } else {
                        false
                    }
                }) {
                    existing_fields.push(field);
                } else {
                    fields.push(Field::OneOf {
                        name: oneof.to_owned(),
                        fields: vec![field],
                    });
                }
            } else {
                fields.push(Field::Normal(field));
            }
        }

        // TODO: Make sure optional and required stuff makes sense between proto2/proto3
        for field in fields {
            self.types.push(format!(
                "{},",
                field.type_definition(self.export_map, &self.file_descriptor_proto)
            ));

            json_type.push(format!(
                "{},",
                field.json_type(self.export_map, &self.file_descriptor_proto)
            ));

            encode_lines.append(&mut field.encode(self.export_map, &self.file_descriptor_proto));
            encode_lines.blank();

            json_encode_lines
                .append(&mut field.json_encode(self.export_map, &self.file_descriptor_proto));
            json_encode_lines.blank();

            default_lines.push(format!(
                "{} = {},",
                field.name(),
                field.default(self.export_map, &self.file_descriptor_proto)
            ));

            for inner_field in field.inner_fields() {
                let decoded = decode_field(
                    &format!("self.{}", field.name()),
                    inner_field,
                    self.export_map,
                    &self.file_descriptor_proto,
                    matches!(field, Field::OneOf { .. }),
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
                .replace("<json_decode>", "-- NYI")
                .replace("<json_type>", &json_type.build()),
        );
        self.implementations.blank();

        for nested_message in &message.nested_type {
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

        let mut to_number = IfBuilder::new();
        to_number.indent_n(2);

        for (index, field) in descriptor.value.iter().enumerate() {
            self.types.push(format!(
                "{}\"{}\"",
                if index == 0 { "" } else { "| " },
                field.name()
            ));

            from_number.add_condition(&format!("value == {}", field.number()), |builder| {
                builder.push(format!("return \"{}\"", field.name()));
            });

            to_number.add_condition(&format!("self == \"{}\"", field.name()), |builder| {
                builder.push(format!("return {}", field.number()));
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
                ),
        );
        self.implementations.blank();
    }
}
