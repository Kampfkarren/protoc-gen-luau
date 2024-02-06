use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};

use prost_types::{
    compiler::{
        code_generator_response::{Feature, File},
        CodeGeneratorRequest, CodeGeneratorResponse,
    },
    field_descriptor_proto::{Label, Type},
    DescriptorProto, EnumDescriptorProto, FileDescriptorProto,
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
			else
				error("Unsupported wire type: " .. wireType)
			end
		end

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

        File {
            name: Some(
                PathBuf::from(self.file_descriptor_proto.name())
                    .with_extension("luau")
                    .to_string_lossy()
                    .into_owned(),
            ),
            content: Some(contents.build()),
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

        let mut default_lines = StringBuilder::new();
        default_lines.indent_n(3);

        let mut encode_lines = StringBuilder::new();
        encode_lines.indent_n(2);

        let mut varint_fields: BTreeMap<i32, String> = BTreeMap::new();
        let mut len_fields: BTreeMap<i32, String> = BTreeMap::new();

        // TODO: Make sure optional and required stuff makes sense between proto2/proto3
        for field in &message.field {
            let mut var_type;
            let mut default;

            let mut encode_builder = StringBuilder::new();
            let mut encode_check = None;

            let is_optional = field.oneof_index.is_some() || field.r#type() == Type::Message;

            let field_name = field.name();
            let number = field.number();

            let decode_fields;
            let decode_value;

            match field.r#type() {
                Type::Int32
                | Type::Uint32
                // | Type::Sint32
                // | Type::Fixed32
                // | Type::Sfixed32
                // | Type::Double
                // | Type::Float
                 => {
                    var_type = "number".to_owned();
                    default = "0".to_owned();

                    encode_builder.push(format!("output, cursor = proto.writeTag(output, cursor, {number}, proto.wireTypes.varint)"));
                    encode_builder.push("output, cursor = proto.writeVarInt(output, cursor, <value>)");

                    decode_fields = &mut varint_fields;
                    decode_value = "value".to_owned();
                }

                Type::String => {
                    var_type = "string".to_owned();
                    default = "\"\"".to_owned();

                    encode_builder.push(format!("output, cursor = proto.writeTag(output, cursor, {number}, proto.wireTypes.lengthDelimited)"));
                    encode_builder.push("output, cursor = proto.writeString(output, cursor, <value>)");

                    decode_fields = &mut len_fields;
                    decode_value = "buffer.tostring(value)".to_owned();
                }

                Type::Bool => {
                    var_type = "boolean".to_owned();
                    default = "false".to_owned();

                    encode_builder.push(format!("output, cursor = proto.writeTag(output, cursor, {number}, proto.wireTypes.varint)"));
                    encode_builder.push("output, cursor = proto.writeVarInt(output, cursor, if <value> then 1 else 0)");

                    decode_fields = &mut varint_fields;
                    decode_value = "value ~= 0".to_owned();
                }

                Type::Bytes => {
                    var_type = "buffer".to_owned();
                    default = "buffer.create(0)".to_owned();

                    encode_check = Some(format!("if buffer.len(self.{field_name}) > 0 then"));

                    encode_builder.push(format!("output, cursor = proto.writeTag(output, cursor, {number}, proto.wireTypes.lengthDelimited)"));
                    encode_builder.push("output, cursor = proto.writeBuffer(output, cursor, <value>)");

                    decode_fields = &mut len_fields;
                    decode_value = "value".to_owned();
                }

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

                    var_type = if package == self.file_descriptor_proto.package() {
                        just_type.to_owned()
                    } else {
                        let export = self.export_map
                            .get(&format!("{}.{}", package, just_type))
                            .unwrap_or_else(|| panic!("couldn't find export {package}.{just_type}"));

                        if export.path == Path::new(self.file_descriptor_proto.name()).with_extension("") {
                            format!("{}{just_type}", export.prefix)
                        } else {
                            format!("{}.{}{just_type}", file_path_export_name(&export.path), export.prefix)
                        }
                    };

                    if field.r#type() == Type::Enum {
                        default = format!("assert({var_type}.fromNumber(0), \"NYI: Proto2 first enum variants as defaults\")");
                        encode_builder.push(format!("output, cursor = proto.writeTag(output, cursor, {number}, proto.wireTypes.varint)"));
                        encode_builder.push(format!("output, cursor = proto.writeVarInt(output, cursor, if typeof(<value>) == \"number\" then <value> else {var_type}.toNumber(<value>))"));

                        decode_fields = &mut varint_fields;
                        decode_value = format!("{var_type}.fromNumber(value) or value");
                    } else {
                        default = "nil".to_owned();

                        encode_builder.push(format!("local encoded = {var_type}.encode(<value>)"));
                        encode_builder.push(format!("output, cursor = proto.writeTag(output, cursor, {number}, proto.wireTypes.lengthDelimited)"));
                        encode_builder.push("output, cursor = proto.writeVarInt(output, cursor, buffer.len(encoded))");
                        encode_builder.push("output, cursor = proto.writeBuffer(output, cursor, encoded)");

                        decode_fields = &mut len_fields;
                        decode_value = format!("{var_type}.decode(value)");
                    }
                }

                other => unimplemented!("Unsupported type: {other:?}"),
            };

            let encode_check = encode_check.unwrap_or_else(|| {
                format!(
                    "if self.{field_name} ~= {default} then",
                    field_name = field_name,
                    default = default
                )
            });

            if is_optional {
                encode_lines.push(format!("if self.{field_name} ~= nil then"));
                encode_lines.indent();
            }

            // TODO: proto2 stuff (required/optional)
            if field.label.is_some() && field.label() == Label::Repeated {
                var_type = format!("{{ {var_type} }}");
                default = "{}".to_owned();

                encode_lines.push(format!("for _, value in self.{field_name} do"));
                encode_lines.indent();

                encode_builder.replace("<value>", "value");
                encode_lines.append(&mut encode_builder);

                encode_lines.dedent();
                encode_lines.push("end");

                let decode_lines = vec![
                    format!("if self.{field_name} == nil then"),
                    format!("\t\tself.{field_name} = {{}}"),
                    "\tend".to_owned(),
                    format!("\tassert(self.{field_name} ~= nil, \"Luau\")\n"),
                    format!("\ttable.insert(self.{field_name}, {decode_value})"),
                ];

                decode_fields.insert(number, decode_lines.join("\n"));
            } else {
                if !is_optional {
                    encode_lines.push(encode_check);
                    encode_lines.indent();
                }

                encode_builder.replace("<value>", &format!("self.{field_name}"));
                encode_lines.append(&mut encode_builder);

                if !is_optional {
                    encode_lines.dedent();
                    encode_lines.push("end");
                }

                decode_fields.insert(number, format!("self.{field_name} = {decode_value}"));
            }

            if is_optional {
                encode_lines.dedent();
                encode_lines.push("end");
            }

            encode_lines.blank();

            // TODO: proto2?
            // TODO: "in proto3 the default representation for all user-defined message types is Option<T>"
            if is_optional {
                var_type.push('?');
                default_lines.push(format!("{field_name} = nil,"));
            } else {
                default_lines.push(format!("{field_name} = {default},"));
            }

            self.types.push(format!("{field_name}: {var_type},"));
        }

        self.types.dedent();
        self.types.push("}");
        self.types.blank();

        self.implementations.push(
            MESSAGE
                .replace("<name>", &name)
                .replace("<default>", &default_lines.build())
                .replace("<encode>", &encode_lines.build())
                .replace("<decode_varint>", &create_decoder(varint_fields))
                .replace("<decode_len>", &create_decoder(len_fields)),
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
