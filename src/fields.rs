use std::{borrow::Cow, collections::HashMap};

use typed_path::UnixPath as Path;

use prost_types::{
    field_descriptor_proto::{Label, Type},
    FieldDescriptorProto, FileDescriptorProto,
};

use crate::{
    generator::{file_path_export_name, ExportMap, MapType},
    if_builder::IfBuilder,
    string_builder::StringBuilder,
};

pub struct FieldGenerator<'a> {
    pub field_kind: FieldKind<'a>,
    pub export_map: &'a ExportMap,
    pub base_file: &'a FileDescriptorProto,
}

#[derive(Debug)]
pub enum FieldKind<'a> {
    Single(&'a FieldDescriptorProto),
    OneOf {
        name: String,
        fields: Vec<&'a FieldDescriptorProto>,
    },
}

impl FieldGenerator<'_> {
    // In a simple sense: will this be T? or T
    fn has_presence(&self) -> bool {
        match &self.field_kind {
            FieldKind::Single(field) => {
                if self.map_type().is_some() {
                    return false;
                }

                if field.label == Some(Label::Repeated as i32) {
                    return false;
                }

                field.proto3_optional() || matches!(field.r#type(), Type::Message)
            }

            FieldKind::OneOf { .. } => true,
        }
    }

    pub fn name(&self) -> &str {
        match &self.field_kind {
            FieldKind::Single(field) => field.name(),
            FieldKind::OneOf { name, .. } => name,
        }
    }

    pub fn type_definition_no_presence(&self) -> String {
        match &self.field_kind {
            FieldKind::Single(field) => {
                if let Some(map_type) = self.map_type() {
                    format!(
                        "{{ [{}]: {} }}",
                        type_definition_of_field_descriptor(
                            &map_type.key,
                            self.export_map,
                            self.base_file
                        ),
                        type_definition_of_field_descriptor(
                            &map_type.value,
                            self.export_map,
                            self.base_file
                        ),
                    )
                } else {
                    let definition =
                        type_definition_of_field_descriptor(field, self.export_map, self.base_file);

                    if field.label.is_some() && field.label() == Label::Repeated {
                        format!("{{ {definition} }}")
                    } else {
                        definition
                    }
                }
            }

            FieldKind::OneOf { fields, .. } => {
                let variants = fields
                    .iter()
                    .map(|field| {
                        format!(
                            "{{ type: \"{}\", value: {} }}",
                            field.name(),
                            type_definition_of_field_descriptor(
                                field,
                                self.export_map,
                                self.base_file
                            )
                        )
                    })
                    .collect::<Vec<_>>();

                format!("({})", variants.join(" | "))
            }
        }
    }

    pub fn type_definition(&self) -> String {
        let mut definition = self.type_definition_no_presence();

        if self.has_presence() {
            definition.push('?');
        }

        definition
    }

    fn json_map_type_definition(&self) -> String {
        let map_type = self.map_type().unwrap();

        format!(
            "{{ [string]: {} }}",
            json_type_definition_of_field_descriptor(
                &map_type.value,
                self.export_map,
                self.base_file
            )
        )
    }

    pub fn json_type_and_names(&self) -> StringBuilder {
        match &self.field_kind {
            FieldKind::Single(field) => {
                let name = json_name(field);

                if self.map_type().is_some() {
                    StringBuilder::from(format!("{name}: {}?,", self.json_map_type_definition()))
                } else {
                    let mut definition = json_type_definition_of_field_descriptor(
                        field,
                        self.export_map,
                        self.base_file,
                    );

                    if field.label.is_some() && field.label() == Label::Repeated {
                        definition = format!("{{ {definition} }}");
                    }

                    definition.push('?');
                    StringBuilder::from(format!("{name}: {definition},"))
                }
            }

            FieldKind::OneOf { fields, .. } => {
                let mut json_type_and_names = StringBuilder::new();

                for field in fields {
                    json_type_and_names.push(format!(
                        "{}: {}?,",
                        json_name(field),
                        json_type_definition_of_field_descriptor(
                            field,
                            self.export_map,
                            self.base_file
                        )
                    ));
                }

                json_type_and_names
            }
        }
    }

    pub fn should_encode(&self) -> String {
        let this = format!("self.{}", self.name());

        if self.has_presence() {
            return format!("{this} ~= nil");
        }

        match &self.field_kind {
            FieldKind::OneOf { .. } => unreachable!("OneOf has presence"),

            FieldKind::Single(field) => {
                if self.map_type().is_some() {
                    return format!("{this} ~= nil and next({this}) ~= nil");
                }

                if field.label.is_some() && field.label() == Label::Repeated {
                    return format!("{this} ~= nil and #{this} > 0");
                }

                // TODO: Remove default branch and explicitly type everything out
                match field.r#type() {
                    Type::Int32
                    | Type::Uint32
                    | Type::Int64
                    | Type::Uint64
                    | Type::Sint32
                    | Type::Sint64
                    | Type::Sfixed32
                    | Type::Sfixed64
                    | Type::Fixed32
                    | Type::Fixed64
                    | Type::Float
                    | Type::Double => {
                        format!("{this} ~= nil and {this} ~= 0")
                    }
                    Type::String => format!("{this} ~= nil and {this} ~= \"\""),
                    Type::Bool => this,
                    Type::Bytes => format!("{this} ~= nil and buffer.len({this}) > 0"),
                    Type::Enum => format!(
                        "{this} ~= nil and ({this} ~= nil and {this} ~= 0 or {this} ~= {}.fromNumber(0))",
                        type_definition_of_field_descriptor(field, self.export_map, self.base_file)
                    ),
                    Type::Message => unreachable!("Message has presence"),

                    Type::Group => unimplemented!("Group"),
                }
            }
        }
    }

    pub fn encode(&self) -> StringBuilder {
        let this = format!("self.{}", self.name());

        let mut encode = StringBuilder::new();
        encode.push(format!("if {} then", self.should_encode()));

        match &self.field_kind {
            FieldKind::Single(field) => {
                if let Some(map_type) = self.map_type() {
                    // Maps are { 1: key, 2: value }
                    encode.push(format!("for key, value in {this} do"));

                    encode.push("local mapBuffer = buffer.create(0)");
                    encode.push("local mapCursor = 0");

                    encode.push(
                        encode_field_descriptor_ignore_repeated(
                            &map_type.key,
                            self.export_map,
                            self.base_file,
                            "key",
                        )
                        .replace("cursor", "mapCursor")
                        .replace("output", "mapBuffer"),
                    );

                    encode.push(
                        encode_field_descriptor_ignore_repeated(
                            &map_type.value,
                            self.export_map,
                            self.base_file,
                            "value",
                        )
                        .replace("cursor", "mapCursor")
                        .replace("output", "mapBuffer"),
                    );

                    encode.push(format!("output, cursor = proto.writeTag(output, cursor, {}, proto.wireTypes.lengthDelimited)", field.number()));
                    encode.push(
                        "output, cursor = proto.writeBuffer(output, cursor, mapBuffer, mapCursor)",
                    );

                    encode.push("end");
                } else if field.label.is_some() && field.label() == Label::Repeated {
                    if is_packed(field) {
                        let field_number = field.number();
                        let write_value = encode_field_descriptor_ignore_repeated_instruction(
                            field,
                            self.export_map,
                            self.base_file,
                            "value",
                        )
                        .replace("cursor", "packedCursor")
                        .replace("output", "packedBuffer");

                        encode.push(indoc::formatdoc!{"
                            local packedBuffer = buffer.create(0)
                            local packedCursor = 0

                            for _, value in {this} do
                                {write_value}
                            end

                            output, cursor = proto.writeTag(output, cursor, {field_number}, proto.wireTypes.lengthDelimited)
                            output, cursor = proto.writeBuffer(output, cursor, packedBuffer, packedCursor)
                        "});
                    } else {
                        encode.push(format!("for _, value in {this} do"));
                        encode.indent();

                        encode.push(encode_field_descriptor_ignore_repeated(
                            field,
                            self.export_map,
                            self.base_file,
                            "value",
                        ));

                        encode.dedent();
                        encode.push("end");
                    }
                } else {
                    encode.push(encode_field_descriptor_ignore_repeated(
                        field,
                        self.export_map,
                        self.base_file,
                        &this,
                    ));
                }
            }

            FieldKind::OneOf { fields, .. } => {
                let mut if_builder = IfBuilder::new();

                for field in fields {
                    if_builder.add_condition(
                        &format!("{this}.type == \"{}\"", field.name()),
                        |builder| {
                            builder.push(encode_field_descriptor_ignore_repeated(
                                field,
                                self.export_map,
                                self.base_file,
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

    pub fn json_encode(&self) -> StringBuilder {
        let this = format!("self.{}", self.name());

        let mut json_encode = StringBuilder::new();
        json_encode.push(format!("if {} then", self.should_encode()));

        match &self.field_kind {
            FieldKind::Single(field) => {
                let output = format!("output.{}", json_name(field));

                if let Some(map_type) = self.map_type() {
                    json_encode.push(format!(
                        "local newOutput: {} = {{}}",
                        self.json_map_type_definition()
                    ));
                    json_encode.push(format!("for key, value in {this} do"));
                    json_encode.push(format!(
                        "newOutput[{}] = {}",
                        json_key_to_string(&map_type.key).encode,
                        json_encode_instruction_field_descriptor_ignore_repeated(
                            &map_type.value,
                            self.export_map,
                            self.base_file,
                            "value"
                        )
                    ));
                    json_encode.push("end");
                    json_encode.push(format!("{output} = newOutput"));
                } else if field.label.is_some() && field.label() == Label::Repeated {
                    json_encode.push(format!(
                        "local newOutput: {{ {} }} = {{}}",
                        json_type_definition_of_field_descriptor(
                            field,
                            self.export_map,
                            self.base_file
                        )
                    ));
                    json_encode.push(format!("for _, value in {this} do"));
                    json_encode.push(format!(
                        "table.insert(newOutput, {})",
                        json_encode_instruction_field_descriptor_ignore_repeated(
                            field,
                            self.export_map,
                            self.base_file,
                            "value"
                        )
                    ));
                    json_encode.push("end");
                    json_encode.push(format!("{output} = newOutput"));
                } else {
                    json_encode.push(format!(
                        "{output} = {}",
                        json_encode_instruction_field_descriptor_ignore_repeated(
                            field,
                            self.export_map,
                            self.base_file,
                            &this
                        )
                    ));
                }
            }

            FieldKind::OneOf { fields, .. } => {
                let mut if_builder = IfBuilder::new();

                for field in fields {
                    if_builder.add_condition(
                        &format!("{this}.type == \"{}\"", field.name()),
                        |builder| {
                            builder.push(format!(
                                "output.{} = {}",
                                json_name(field),
                                json_encode_instruction_field_descriptor_ignore_repeated(
                                    field,
                                    self.export_map,
                                    self.base_file,
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

    // TODO: For here and json_encode, they need to be camelCase
    pub fn json_decode(&self) -> StringBuilder {
        let mut json_decode = StringBuilder::new();

        for inner_field in self.inner_fields() {
            let real_name = inner_field.name();
            let json_name = json_name(inner_field);
            let luau_types = valid_json_types_of_field_descriptor(inner_field);

            let mut decode_name = |input_name: &str| {
                json_decode.push(format!("if input.{input_name} ~= nil then"));

                if let Some(map_type) = self.map_type() {
                    json_decode.push(format!(
                        "local newOutput: {} = {{}}",
                        self.type_definition()
                    ));
                    json_decode.push(format!("for key, value in input.{input_name} do"));

                    let key_luau_types = valid_json_types_of_field_descriptor(&map_type.key);
                    let key_type_checks = key_luau_types
                        .iter()
                        .map(|luau_type| {
                            format!(
                                "typeof(key) ~= \"{luau_type}\"",
                                luau_type = luau_type
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" and ");
                    json_decode.push(format!("if {key_type_checks} then"));
                    json_decode.push(format!(
                        "  error(\"Invalid value provided for field {real_name}\")"
                    ));
                    json_decode.push("end");

                    let value_luau_types = valid_json_types_of_field_descriptor(&map_type.value);
                    let value_type_checks = value_luau_types
                        .iter()
                        .map(|luau_type| {
                            format!(
                                "typeof(value) ~= \"{luau_type}\"",
                                luau_type = luau_type
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" and ");
                    json_decode.push(format!("if {value_type_checks} then"));
                    json_decode.push(format!(
                        "  error(\"Invalid value provided for field {real_name}\")"
                    ));
                    json_decode.push("end");

                    json_decode.push(format!(
                        "newOutput[{}] = {}",
                        json_key_to_string(&map_type.key).decode,
                        json_decode_instruction_field_descriptor_ignore_repeated(
                            &map_type.value,
                            self.export_map,
                            self.base_file,
                            "value"
                        )
                    ));
                    json_decode.push("end");
                    json_decode.blank();
                    json_decode.push(format!("self.{real_name} = newOutput"));
                } else if inner_field.label.is_some() && inner_field.label() == Label::Repeated {
                    json_decode.push(format!(
                        "local newOutput: {} = {{}}",
                        self.type_definition()
                    ));
                    json_decode.push(format!("for _, value in input.{input_name} do"));
                    let type_checks = luau_types
                        .iter()
                        .map(|luau_type| {
                            format!(
                                "typeof(value) ~= \"{luau_type}\"",
                                luau_type = luau_type
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" and ");
                    json_decode.push(format!("if {type_checks} then"));
                    json_decode.push(format!(
                        "  error(\"Invalid value provided for field {real_name}\")"
                    ));
                    json_decode.push("end");

                    json_decode.push(format!(
                        "table.insert(newOutput, {})",
                        json_decode_instruction_field_descriptor_ignore_repeated(
                            inner_field,
                            self.export_map,
                            self.base_file,
                            "value"
                        )
                    ));
                    json_decode.push("end");
                    json_decode.blank();
                    json_decode.push(format!("self.{real_name} = newOutput"));
                } else {
                    let json_decode_instruction =
                        json_decode_instruction_field_descriptor_ignore_repeated(
                            inner_field,
                            self.export_map,
                            self.base_file,
                            &format!("input.{input_name}"),
                        );

                    // if ["number", "string", "boolean"].contains(&luau_type.as_str()) {
                    //     json_decode.push(format!(
                    //         "if typeof(input.{input_name}) ~= \"{luau_type}\" then"
                    //     ));
                    //     json_decode.push(format!(
                    //         "  error(\"Invalid value provided for field {real_name}\")"
                    //     ));
                    //     json_decode.push("end");
                    // }
                    let type_checks = luau_types
                        .iter()
                        .map(|luau_type| {
                            format!(
                                "typeof(input.{input_name}) ~= \"{luau_type}\"",
                                luau_type = luau_type
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" and ");
                    json_decode.push(format!("if {type_checks} then"));
                    json_decode.push(format!(
                        "  error(\"Invalid value provided for field {real_name}\")"
                    ));
                    json_decode.push("end");

                    if let FieldKind::OneOf {
                        name: oneof_name, ..
                    } = &self.field_kind
                    {
                        json_decode.push(format!(
                        "self.{oneof_name} = {{ type = \"{real_name}\", value = {json_decode_instruction} }}",
                    ));
                    } else {
                        json_decode.push(format!("self.{real_name} = {json_decode_instruction}"));
                    }
                }

                json_decode.push("end");
                json_decode.blank();
            };

            decode_name(real_name);

            if real_name != json_name {
                decode_name(&json_name);
            }
        }

        json_decode
    }

    pub fn inner_fields(&self) -> Vec<&FieldDescriptorProto> {
        match &self.field_kind {
            FieldKind::Single(field) => vec![field],
            FieldKind::OneOf { fields, .. } => fields.clone(),
        }
    }

    pub fn map_type(&self) -> Option<&MapType> {
        let FieldKind::Single(field) = &self.field_kind else {
            return None;
        };

        let type_name = field.type_name();
        if type_name.is_empty() {
            return None;
        }

        assert!(
            type_name.starts_with('.'),
            "NYI: Relative type names: {type_name:?}"
        );

        let export = self
            .export_map
            .get(&type_name[1..])
            .or_else(|| self.export_map.get(type_name))?;

        export.map.as_ref()
    }

    pub fn default(&self) -> Cow<'static, str> {
        if self.has_presence() {
            return "nil".into();
        }

        match self.field_kind {
            FieldKind::Single(field) => {
                default_of_type_descriptor_ignore_presence(field, self.export_map, self.base_file)
            }

            FieldKind::OneOf { .. } => "nil".into(),
        }
    }
}

fn type_definition_of_field_descriptor(
    field: &FieldDescriptorProto,
    export_map: &ExportMap,
    base_file: &FileDescriptorProto,
) -> String {
    match field.r#type() {
        Type::Int32
        | Type::Uint32
        | Type::Int64
        | Type::Uint64
        | Type::Fixed32
        | Type::Fixed64
        | Type::Sint32
        | Type::Sint64
        | Type::Sfixed32
        | Type::Sfixed64
        | Type::Float
        | Type::Double => "number".to_owned(),
        Type::String => "string".to_owned(),
        Type::Bool => "boolean".to_owned(),
        Type::Bytes => "buffer".to_owned(),
        Type::Enum | Type::Message => {
            let original_type_name = field.type_name();
            assert!(
                original_type_name.starts_with('.'),
                "NYI: Relative type names: {original_type_name}"
            );

            let type_name = &original_type_name[1..];

            let mut segments: Vec<&str> = type_name.split('.').collect();
            let just_type = segments.pop().unwrap();
            let package = segments.join(".");

            let export = export_map
                .get(&format!("{package}.{just_type}"))
                .or_else(|| export_map.get(original_type_name))
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

        Type::Group => unimplemented!("Group"),
    }
}

fn json_type_definition_of_field_descriptor(
    field: &FieldDescriptorProto,
    export_map: &ExportMap,
    base_file: &FileDescriptorProto,
) -> String {
    match field.r#type() {
        Type::Float | Type::Double => "(number | string)".to_owned(),
        Type::Bytes => "string".to_owned(),
        _ => type_definition_of_field_descriptor(field, export_map, base_file),
    }
}

fn valid_json_types_of_field_descriptor(field: &FieldDescriptorProto) -> &'static [&'static str] {
    match field.r#type() {
        Type::Int32
        | Type::Uint32
        | Type::Int64
        | Type::Uint64
        | Type::Fixed32
        | Type::Fixed64
        | Type::Sint32
        | Type::Sint64
        | Type::Sfixed32
        | Type::Sfixed64
        | Type::Float
        | Type::Double => &["number", "string"],
        Type::String => &["string"],
        Type::Bool => &["boolean"],
        Type::Bytes => &["string"],
        Type::Enum => &["number", "string"],
        Type::Message => &["table"],
        Type::Group => unimplemented!("Group"),
    }
}

#[derive(Clone, Copy)]
pub enum WireType {
    Varint,
    LengthDelimited,
    I32,
    I64,
}

pub fn wire_type_of_field_descriptor(field: &FieldDescriptorProto) -> WireType {
    match field.r#type() {
        Type::Int32
        | Type::Uint32
        | Type::Int64
        | Type::Uint64
        | Type::Sint32
        | Type::Sint64
        | Type::Enum
        | Type::Bool => WireType::Varint,
        Type::Float | Type::Fixed32 | Type::Sfixed32 => WireType::I32,
        Type::Double | Type::Fixed64 | Type::Sfixed64 => WireType::I64,
        Type::String | Type::Bytes | Type::Message => WireType::LengthDelimited,
        Type::Group => unimplemented!("Group"),
    }
}

fn encode_field_descriptor_ignore_repeated_instruction(
    field: &FieldDescriptorProto,
    export_map: &ExportMap,
    base_file: &FileDescriptorProto,
    value_var: &str,
) -> String {
    match field.r#type() {
        Type::Int32 | Type::Uint32 | Type::Int64 | Type::Uint64 =>
            format!("output, cursor = proto.writeVarInt(output, cursor, {value_var})"),

        Type::Sint32 | Type::Sint64 => format!(
            "output, cursor = proto.writeVarInt(output, cursor, proto.encodeZigZag({value_var}))",
        ),

        Type::Float => format!("output, cursor = proto.writeFloat(output, cursor, {value_var})"),
        Type::Double => format!("output, cursor = proto.writeDouble(output, cursor, {value_var})"),
        Type::String => format!("output, cursor = proto.writeString(output, cursor, {value_var})"),

        Type::Bool => format!(
            "output, cursor = proto.writeVarInt(output, cursor, if {value_var} then 1 else 0)",
        ),

        Type::Bytes => format!("output, cursor = proto.writeBuffer(output, cursor, {value_var}, buffer.len({value_var}))"),

        Type::Enum => format!(
            // :: any cast because Luau is bad with string unions
            "output, cursor = proto.writeVarInt(output, cursor, {}.toNumber({value_var} :: any))",
            type_definition_of_field_descriptor(field, export_map, base_file)
        ),

        Type::Message => unimplemented!(),

        Type::Fixed32 | Type::Sfixed32 => format!("output, cursor = proto.writeFixed32(output, cursor, {value_var})"),
        Type::Fixed64 | Type::Sfixed64 => format!("output, cursor = proto.writeFixed64(output, cursor, {value_var})"),

        Type::Group => unimplemented!("Group"),
    }
}

fn encode_field_descriptor_ignore_repeated(
    field: &FieldDescriptorProto,
    export_map: &ExportMap,
    base_file: &FileDescriptorProto,
    value_var: &str,
) -> String {
    if field.r#type() == Type::Message {
        return [
            format!(
                "local encoded = {}.encode({value_var})",
                type_definition_of_field_descriptor(field, export_map, base_file)
            ),
            format!(
                "output, cursor = proto.writeTag(output, cursor, {}, proto.wireTypes.lengthDelimited)",
                field.number()
            ),
            "output, cursor = proto.writeBuffer(output, cursor, encoded, buffer.len(encoded))".to_owned(),
        ]
        .join("\n");
    }

    let setup = match wire_type_of_field_descriptor(field) {
        WireType::Varint => format!(
            "output, cursor = proto.writeTag(output, cursor, {}, proto.wireTypes.varint)",
            field.number()
        ),
        WireType::LengthDelimited => format!(
            "output, cursor = proto.writeTag(output, cursor, {}, proto.wireTypes.lengthDelimited)",
            field.number()
        ),
        WireType::I32 => format!(
            "output, cursor = proto.writeTag(output, cursor, {}, proto.wireTypes.i32)",
            field.number()
        ),
        WireType::I64 => format!(
            "output, cursor = proto.writeTag(output, cursor, {}, proto.wireTypes.i64)",
            field.number()
        ),
    };

    format!(
        "{setup}\n{}",
        encode_field_descriptor_ignore_repeated_instruction(
            field, export_map, base_file, value_var
        )
    )
}

fn json_encode_instruction_field_descriptor_ignore_repeated(
    field: &FieldDescriptorProto,
    export_map: &ExportMap,
    base_file: &FileDescriptorProto,
    value_var: &str,
) -> String {
    match field.r#type() {
        Type::Int32
        | Type::Int64
        | Type::Uint32
        | Type::Uint64
        | Type::Fixed32
        | Type::Fixed64
        | Type::Sint32
        | Type::Sint64
        | Type::Sfixed32
        | Type::Sfixed64
        | Type::Bool
        | Type::String => value_var.to_owned(),
        Type::Float | Type::Double => format!("proto.json.serializeNumber({value_var})"),
        Type::Bytes => format!("proto.json.serializeBuffer({value_var})"),
        Type::Enum => format!(
            // :: any cast because Luau is bad with string unions
            "if typeof({value_var}) == \"number\" then {value_var} else {}.toNumber({value_var} :: any)",
            type_definition_of_field_descriptor(field, export_map, base_file)
        ),
        Type::Message => format!(
            "{}.jsonEncode({value_var})",
            type_definition_of_field_descriptor(field, export_map, base_file)
        ),
        Type::Group => unimplemented!("Group"),
    }
}

fn json_decode_instruction_field_descriptor_ignore_repeated(
    field: &FieldDescriptorProto,
    export_map: &ExportMap,
    base_file: &FileDescriptorProto,
    value_var: &str,
) -> String {
    match field.r#type() {
        Type::Int32
        | Type::Int64
        | Type::Uint32
        | Type::Uint64
        | Type::Fixed32
        | Type::Fixed64
        | Type::Sfixed32
        | Type::Sfixed64
        | Type::Sint32
        | Type::Sint64
        | Type::Bool
        | Type::String => value_var.to_owned(),
        Type::Float | Type::Double => format!("proto.json.deserializeNumber({value_var})"),
        Type::Bytes => format!("proto.json.deserializeBuffer({value_var})"),
        Type::Enum => format!(
            "if typeof({value_var}) == \"number\" then ({qualified_enum}.fromNumber({value_var}) \
                or {value_var}) else ({qualified_enum}.fromName({value_var}) or {value_var})",
            qualified_enum = type_definition_of_field_descriptor(field, export_map, base_file)
        ),
        Type::Message => format!(
            "{}.jsonDecode({value_var})",
            type_definition_of_field_descriptor(field, export_map, base_file)
        ),
        Type::Group => unimplemented!("Group"),
    }
}

struct JsonKeyToString {
    encode: &'static str,
    decode: &'static str,
}
fn json_key_to_string(field: &FieldDescriptorProto) -> JsonKeyToString {
    match field.r#type() {
        Type::Bool => JsonKeyToString {
            encode: "tostring(key)",
            decode: "if key == \"true\" then true else false",
        },

        Type::String => JsonKeyToString {
            encode: "key",
            decode: "key",
        },

        Type::Int32
        | Type::Int64
        | Type::Uint32
        | Type::Uint64
        | Type::Fixed32
        | Type::Fixed64
        | Type::Sint32
        | Type::Sint64
        | Type::Sfixed32
        | Type::Sfixed64 => JsonKeyToString {
            encode: "tostring(key)",
            decode: "(assert(tonumber(key), \"Invalid number provided as key\"))",
        },

        Type::Double | Type::Float | Type::Group | Type::Message | Type::Bytes | Type::Enum => {
            unreachable!("Invalid type for map key")
        }
    }
}

fn json_name(field: &FieldDescriptorProto) -> Cow<str> {
    if let Some(json_name) = &field.json_name {
        json_name.into()
    } else {
        heck::AsLowerCamelCase(field.name()).to_string().into()
    }
}

fn default_of_type_descriptor_ignore_presence(
    field: &FieldDescriptorProto,
    export_map: &ExportMap,
    base_file: &FileDescriptorProto,
) -> Cow<'static, str> {
    if field.label.is_some() && field.label() == Label::Repeated {
        return "{}".into();
    }

    match field.r#type() {
        Type::Int32
        | Type::Uint32
        | Type::Int64
        | Type::Uint64
        | Type::Fixed32
        | Type::Fixed64
        | Type::Sint32
        | Type::Sint64
        | Type::Sfixed32
        | Type::Sfixed64
        | Type::Float
        | Type::Double => "0".into(),
        Type::String => "\"\"".into(),
        Type::Bool => "false".into(),
        Type::Bytes => "buffer.create(0)".into(),
        // proto2: Enums default to first value
        Type::Enum => format!(
            "assert({}.fromNumber(0), \"Enum has no 0 default\")",
            type_definition_of_field_descriptor(field, export_map, base_file)
        )
        .into(),
        Type::Message => format!(
            "{}.new()",
            type_definition_of_field_descriptor(field, export_map, base_file)
        )
        .into(),
        Type::Group => unimplemented!("Group"),
    }
}

fn decode_instruction_field_descriptor_ignore_repeated(
    field: &FieldDescriptorProto,
    export_map: &ExportMap,
    base_file: &FileDescriptorProto,
) -> Cow<'static, str> {
    match field.r#type() {
        Type::Uint32
        | Type::Int64
        | Type::Uint64
        | Type::Fixed32
        | Type::Fixed64
        | Type::Sfixed32
        | Type::Sfixed64
        | Type::Float
        | Type::Double
        | Type::Bytes => "value".into(),

        Type::Int32 => "proto.limitInt32(value)".into(),

        Type::Sint32 | Type::Sint64 => "proto.decodeZigZag(value)".into(),

        Type::Bool => "value ~= 0".into(),

        Type::String => "buffer.tostring(value)".into(),

        Type::Enum => format!(
            "({}.fromNumber(value) or value) :: any --[[ Luau: Enums are a string intersection which Luau is quick to dismantle ]]",
            type_definition_of_field_descriptor(field, export_map, base_file)
        )
        .into(),

        Type::Message => format!(
            "{}.decode(value)",
            type_definition_of_field_descriptor(field, export_map, base_file)
        )
        .into(),

        Type::Group => unimplemented!("Group"),
    }
}

// TODO: Variable for "value" instead of replace
pub fn decode_field(
    this: &str,
    field: &FieldDescriptorProto,
    export_map: &ExportMap,
    base_file: &FileDescriptorProto,
    map_type: Option<&MapType>,
    is_oneof: bool,
) -> StringBuilder {
    let mut decode = StringBuilder::new();

    if let Some(map_type) = map_type {
        let map_entry_type = type_definition_of_field_descriptor(field, export_map, base_file);

        let key_default =
            default_of_type_descriptor_ignore_presence(&map_type.key, export_map, base_file);

        let value_default =
            default_of_type_descriptor_ignore_presence(&map_type.value, export_map, base_file);

        // TODO: Type keyDefault and valueDefault
        decode.push(indoc::formatdoc! {"
            local value
            value, cursor = proto.readBuffer(input, cursor)

            local mapEntry = {map_entry_type}.decode(value)

            local keyDefault = {key_default}
            local valueDefault = {value_default}

            {this}[mapEntry.key or keyDefault] = mapEntry.value or valueDefault
        "})
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

            Type::Fixed32 => {
                decode.push("local value");
                decode.push("value, cursor = proto.readFixed32(input, cursor)");
            }

            Type::Fixed64 => {
                decode.push("local value");
                decode.push("value, cursor = proto.readFixed64(input, cursor)");
            }

            Type::Sfixed32 => {
                decode.push("local value");
                decode.push("value, cursor = proto.readSignedFixed32(input, cursor)");
            }

            Type::Sfixed64 => {
                decode.push("local value");
                decode.push("value, cursor = proto.readSignedFixed64(input, cursor)");
            }

            _ => match wire_type_of_field_descriptor(field) {
                WireType::Varint => {
                    decode.push("local value");
                    decode.push("value, cursor = proto.readVarInt(input, cursor)");
                }

                WireType::LengthDelimited => {
                    decode.push("local value");
                    decode.push("value, cursor = proto.readBuffer(input, cursor)");
                }

                WireType::I32 | WireType::I64 => {}
            },
        }

        if field.label.is_some() && field.label() == Label::Repeated {
            decode.push(format!(
                "table.insert({this}, {})",
                decode_instruction_field_descriptor_ignore_repeated(field, export_map, base_file)
            ));
        } else if is_oneof {
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

pub fn is_packed(field_descriptor: &FieldDescriptorProto) -> bool {
    if field_descriptor.label.is_none() || field_descriptor.label() != Label::Repeated {
        return false;
    }

    if !matches!(
        field_descriptor.r#type(),
        Type::Double
            | Type::Float
            | Type::Int32
            | Type::Int64
            | Type::Sint32
            | Type::Sint64
            | Type::Uint32
            | Type::Uint64
            | Type::Fixed32
            | Type::Fixed64
            | Type::Sfixed32
            | Type::Sfixed64
            | Type::Bool
    ) {
        return false;
    }

    match field_descriptor.options {
        // proto2: packed is not default
        Some(ref options) => options.packed != Some(false),
        None => true,
    }
}

pub fn decode_packed(field_descriptor: &FieldDescriptorProto, output: &str) -> String {
    let entry_decode = decode_field(
        output,
        field_descriptor,
        &HashMap::new(),
        &FileDescriptorProto::default(),
        None,
        false,
    )
    .build();

    indoc::formatdoc! {"
        local length
        length, cursor = proto.readVarInt(input, cursor)

        local limit = cursor + length

        while cursor < limit do
            {entry_decode}
        end
    "}
}
