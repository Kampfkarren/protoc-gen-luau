use std::borrow::Cow;

use prost_types::{DescriptorProto, FileDescriptorProto};

pub struct WktJson {
    pub luau_type: &'static str,
    pub code: Cow<'static, str>,
}

impl WktJson {
    pub fn try_create(
        file_descriptor_proto: &FileDescriptorProto,
        message: &DescriptorProto,
    ) -> Option<Self> {
        if file_descriptor_proto.package() != "google.protobuf" {
            return None;
        }

        match message.name() {
            "Any" => Some(WktJson {
                luau_type: "{ [string]: any }",
                code: include_str!("./luau/wkt_mixins/Any.luau").into(),
            }),

            "BytesValue" => Some(WktJson {
                luau_type: "string",
                code: include_str!("./luau/wkt_mixins/BytesValue.luau").into(),
            }),

            "Duration" => Some(WktJson {
                luau_type: "string",
                code: include_str!("./luau/wkt_mixins/Duration.luau").into(),
            }),

            "FieldMask" => Some(WktJson {
                luau_type: "string",
                code: include_str!("./luau/wkt_mixins/FieldMask.luau").into(),
            }),

            "ListValue" => Some(WktJson {
                luau_type: "{ any }",
                code: include_str!("./luau/wkt_mixins/ListValue.luau").into(),
            }),

            "Struct" => Some(WktJson {
                luau_type: "{ [string]: any }",
                code: include_str!("./luau/wkt_mixins/Struct.luau").into(),
            }),

            "Timestamp" => Some(WktJson {
                luau_type: "string",
                code: include_str!("./luau/wkt_mixins/Timestamp.luau").into(),
            }),

            "Value" => Some(WktJson {
                luau_type: "any",
                code: include_str!("./luau/wkt_mixins/Value.luau").into(),
            }),

            "BoolValue" => Some(trivial_value("boolean", "BoolValue")),
            "DoubleValue" => Some(trivial_value("number", "DoubleValue")),
            "FloatValue" => Some(trivial_value("number", "FloatValue")),
            "Int32Value" => Some(trivial_value("number", "Int32Value")),
            "Int64Value" => Some(trivial_value("number", "Int64Value")),
            "StringValue" => Some(trivial_value("string", "StringValue")),
            "UInt32Value" => Some(trivial_value("number", "UInt32Value")),
            "UInt64Value" => Some(trivial_value("number", "UInt64Value")),

            _ => None,
        }
    }
}

const TRIVIAL_VALUE: &str = r#"function _<message_name>Impl.jsonEncode(self: <message_name>): <type>
	return self.value
end

function _<message_name>Impl.jsonDecode(anyValue: any): <message_name>
	local value: <type> = anyValue
	return <message_name>.new({
		value = value
	})
end"#;

fn trivial_value(luau_type: &'static str, message_name: &'static str) -> WktJson {
    WktJson {
        luau_type,
        code: TRIVIAL_VALUE
            .replace("<message_name>", message_name)
            .replace("<type>", luau_type)
            .into(),
    }
}
