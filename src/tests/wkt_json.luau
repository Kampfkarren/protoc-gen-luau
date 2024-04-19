local tests = require("./tests")

local duration = require("./samples/google/protobuf/duration")
local struct = require("./samples/google/protobuf/struct")
local timestamp = require("./samples/google/protobuf/timestamp")
local wrappers = require("./samples/google/protobuf/wrappers")

local assertEquals = tests.assertEquals
local describe = tests.describe
local it = tests.it

describe("JSON round-trips should work for", function()
	describe("Value", function()
		it("with null value", function()
			local json = nil

			local deserialized_value = struct.Value.jsonDecode(json)

			assertEquals(deserialized_value.kind.type, "null_value")
			assertEquals(deserialized_value.kind.value, "NULL_VALUE")

			assertEquals(deserialized_value:jsonEncode(), json)
		end)

		it("with number value", function()
			local json = 3.14

			local deserialized_value = struct.Value.jsonDecode(json)

			assertEquals(deserialized_value.kind.type, "number_value")
			assertEquals(deserialized_value.kind.value, 3.14)

			assertEquals(deserialized_value:jsonEncode(), json)
		end)

		it("with string value", function()
			local json = "Hello, World!"

			local deserialized_value = struct.Value.jsonDecode(json)

			assertEquals(deserialized_value.kind.type, "string_value")
			assertEquals(deserialized_value.kind.value, "Hello, World!")

			assertEquals(deserialized_value:jsonEncode(), json)
		end)

		it("with boolean value", function()
			local json = true

			local deserialized_value = struct.Value.jsonDecode(json)

			assertEquals(deserialized_value.kind.type, "bool_value")
			assertEquals(deserialized_value.kind.value, true)

			assertEquals(deserialized_value:jsonEncode(), json)
		end)

		it("with struct value", function()
			local json = {
				name = {
					family = "Doe",
					given = "John",
				},
				age = 42,
			}

			local deserialized_value = struct.Value.jsonDecode(json)

			assertEquals(deserialized_value.kind.type, "struct_value")
			assertEquals(deserialized_value.kind.value.fields.name.kind.value.fields.family.kind.value, "Doe")

			assertEquals(deserialized_value:jsonEncode(), json)
		end)

		it("with list value", function()
			local json = {
				3.14,
				"Hello, World!",
				true,
			}

			local deserialized_value = struct.Value.jsonDecode(json)

			assertEquals(deserialized_value.kind.type, "list_value")

			assertEquals(deserialized_value.kind.value.values[1].kind.type, "number_value")
			assertEquals(deserialized_value.kind.value.values[1].kind.value, 3.14)

			assertEquals(deserialized_value.kind.value.values[2].kind.type, "string_value")
			assertEquals(deserialized_value.kind.value.values[2].kind.value, "Hello, World!")

			assertEquals(deserialized_value.kind.value.values[3].kind.type, "bool_value")
			assertEquals(deserialized_value.kind.value.values[3].kind.value, true)

			assertEquals(deserialized_value:jsonEncode(), json)
		end)
	end)

	describe("Timestamp", function()
		it("with seconds", function()
			local json = "1970-01-01T00:00:00Z"

			local deserialized_timestamp = timestamp.Timestamp.jsonDecode(json)

			assertEquals(deserialized_timestamp.seconds, 0)
			assertEquals(deserialized_timestamp.nanos, 0)

			assertEquals(deserialized_timestamp:jsonEncode(), json)
		end)

		it("with seconds and nanos", function()
			local json = "1970-01-01T00:00:00.000000001Z"

			local deserialized_timestamp = timestamp.Timestamp.jsonDecode(json)

			assertEquals(deserialized_timestamp.seconds, 0)
			assertEquals(deserialized_timestamp.nanos, 1)

			assertEquals(deserialized_timestamp:jsonEncode(), json)
		end)
	end)

	describe("Duration", function()
		it("with seconds", function()
			local json = "1s"

			local deserialized_duration = duration.Duration.jsonDecode(json)

			assertEquals(deserialized_duration.seconds, 1)
			assertEquals(deserialized_duration.nanos, 0)

			assertEquals(deserialized_duration:jsonEncode(), json)
		end)

		it("with negative seconds", function()
			local json = "-1s"

			local deserialized_duration = duration.Duration.jsonDecode(json)

			assertEquals(deserialized_duration.seconds, -1)
			assertEquals(deserialized_duration.nanos, 0)

			assertEquals(deserialized_duration:jsonEncode(), json)
		end)

		it("with nanos", function()
			local json = "0.000000001s"

			local deserialized_duration = duration.Duration.jsonDecode(json)

			assertEquals(deserialized_duration.seconds, 0)
			assertEquals(deserialized_duration.nanos, 1)

			assertEquals(deserialized_duration:jsonEncode(), json)
		end)

		it("with seconds and many nanos", function()
			local json = "1.5s"

			local deserialized_duration = duration.Duration.jsonDecode(json)

			assertEquals(deserialized_duration.seconds, 1)
			assertEquals(deserialized_duration.nanos, 500000000)

			assertEquals(deserialized_duration:jsonEncode(), json)
		end)

		it("with seconds and one nano", function()
			local json = "1.000000001s"

			local deserialized_duration = duration.Duration.jsonDecode(json)

			assertEquals(deserialized_duration.seconds, 1)
			assertEquals(deserialized_duration.nanos, 1)

			assertEquals(deserialized_duration:jsonEncode(), json)
		end)

		it("with many negative nanos", function()
			local json = "-0.5s"

			local deserialized_duration = duration.Duration.jsonDecode(json)

			assertEquals(deserialized_duration.seconds, 0)
			assertEquals(deserialized_duration.nanos, -500000000)

			assertEquals(deserialized_duration:jsonEncode(), json)
		end)

		it("with one negative nano", function()
			local json = "-0.000000001s"

			local deserialized_duration = duration.Duration.jsonDecode(json)

			assertEquals(deserialized_duration.seconds, 0)
			assertEquals(deserialized_duration.nanos, -1)

			assertEquals(deserialized_duration:jsonEncode(), json)
		end)
	end)

	describe("wrapper for", function()
		it("bool", function()
			local bool_value = wrappers.BoolValue.new({ value = true })

			local json = bool_value:jsonEncode()
			assertEquals(json, true)

			local deserialized_bool_value = wrappers.BoolValue.jsonDecode(json)
			assertEquals(bool_value, deserialized_bool_value)
		end)

		it("int32", function()
			local int32_value = wrappers.Int32Value.new({ value = 42 })

			local json = int32_value:jsonEncode()
			assertEquals(json, 42)

			local deserialized_int32_value = wrappers.Int32Value.jsonDecode(json)
			assertEquals(int32_value, deserialized_int32_value)
		end)

		it("int64", function()
			local int64_value = wrappers.Int64Value.new({ value = 42 })

			local json = int64_value:jsonEncode()
			assertEquals(json, 42)

			local deserialized_int64_value = wrappers.Int64Value.jsonDecode(json)
			assertEquals(int64_value, deserialized_int64_value)
		end)

		it("uint32", function()
			local uint32_value = wrappers.UInt32Value.new({ value = 42 })

			local json = uint32_value:jsonEncode()
			assertEquals(json, 42)

			local deserialized_uint32_value = wrappers.UInt32Value.jsonDecode(json)
			assertEquals(uint32_value, deserialized_uint32_value)
		end)

		it("uint64", function()
			local uint64_value = wrappers.UInt64Value.new({ value = 42 })

			local json = uint64_value:jsonEncode()
			assertEquals(json, 42)

			local deserialized_uint64_value = wrappers.UInt64Value.jsonDecode(json)
			assertEquals(uint64_value, deserialized_uint64_value)
		end)

		it("float", function()
			local float_value = wrappers.FloatValue.new({ value = 3.14 })

			local json = float_value:jsonEncode()
			assertEquals(json, 3.14)

			local deserialized_float_value = wrappers.FloatValue.jsonDecode(json)
			assertEquals(float_value, deserialized_float_value)
		end)

		it("double", function()
			local double_value = wrappers.DoubleValue.new({ value = 3.14 })

			local json = double_value:jsonEncode()
			assertEquals(json, 3.14)

			local deserialized_double_value = wrappers.DoubleValue.jsonDecode(json)
			assertEquals(double_value, deserialized_double_value)
		end)

		it("string", function()
			local string_value = wrappers.StringValue.new({ value = "Hello, World!" })

			local json = string_value:jsonEncode()
			assertEquals(json, "Hello, World!")

			local deserialized_string_value = wrappers.StringValue.jsonDecode(json)
			assertEquals(string_value, deserialized_string_value)
		end)

		it("bytes", function()
			local bytes_value = wrappers.BytesValue.new({ value = "Hello, World!" })

			local json = bytes_value:jsonEncode()

			-- TODO: should a BytesValue be base64-encoded?
			assertEquals(json, "Hello, World!")

			local deserialized_bytes_value = wrappers.BytesValue.jsonDecode(json)
			assertEquals(bytes_value, deserialized_bytes_value)
		end)
	end)
end)

tests.finish()
