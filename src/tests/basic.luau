--!strict
local tests = require("./tests")

local deeper_include_me2 = require("./samples/deeper/include_me2")
local include_me = require("./samples/include_me")
local kitchen_sink = require("./samples/kitchen_sink")
local recursive = require("./samples/recursive")
local same_package = require("./samples/same_package")

local assertEquals = tests.assertEquals
local describe = tests.describe
local it = tests.it

describe("json deserializing", function()
	it("should deserialize and reserialize primitives", function()
		local json = {
			int32 = 1,

			-- These need to be included because JSON output currently includes enum values.
			aliasedEnum = 0,
			enum = 0,
			nestedEnum = 0,
			enum2 = 0,
		}

		local kitchenSink = kitchen_sink.KitchenSink.jsonDecode(json)

		assertEquals(kitchenSink.int32, 1)
		assertEquals(kitchenSink:jsonEncode(), json)
	end)

	it("should deserialize recursive protos", function()
		local json = {
			b = {
				a = {
					number = 100,
				},
			},
		}

		local decoded = recursive.A.jsonDecode(json)
		assert(decoded.b ~= nil, "No decoded.b")
		assert(decoded.b.a ~= nil, "No decoded.b.a")

		assertEquals(decoded.b.a.number, 100)
	end)
end)

describe("descriptors can return", function()
	it("full message name", function()
		assertEquals(kitchen_sink.KitchenSink.new().descriptor.fullName, "my.package.KitchenSink")
	end)

	it("unqualified message name", function()
		assertEquals(kitchen_sink.KitchenSink.new().descriptor.name, "KitchenSink")
	end)
end)

describe("binary encoding", function()
	it("should encode and decode a message with a single field", function()
		local kitchenSink = kitchen_sink.KitchenSink.new({
			int32 = 1,
		})

		local encoded = kitchenSink:encode()
		local decoded = kitchen_sink.KitchenSink.decode(encoded)

		assertEquals(decoded.int32, 1)
	end)

	it("should encode and decode a complete message", function()
		local kitchenSink = kitchen_sink.KitchenSink.new({
			int32 = 1,
			optionalInt32 = 2,
			oneofValue = { type = "y", value = "hello" },
			enum = "Two",
			enum2 = "One",
			ints = { 1, 2, 3 },
			map = { [true] = "true", [false] = "false" },
			nestedMessage = kitchen_sink.KitchenSink_NestedMessage.new({ x = 7 }),
			includedMessage = include_me.IncludedMessage.new({ x = 8 }),
			superNestedMessage = include_me.IncludedMessage_SuperNestedMessage.new({ y = 9 }),
			nestedEnum = "Zero",
			float = 1.25,
			double = 2.5,
			samePackageNestedMessage = same_package.SamePackageNestedMessage.new({ x = 42 }),
			deeperNestedMessage = deeper_include_me2.DeeperNestedMessage.new({ x = 1, y = "One" }),

			messages = {
				kitchen_sink.KitchenSink_NestedMessage.new({ x = 1 }),
				kitchen_sink.KitchenSink_NestedMessage.new({ x = 2 }),
			},

			sint32 = 1,
			sint64 = 2,

			fixed32 = 1,
			fixed64 = 2,

			sfixed32 = 1,

			-- sfixed64 isn't yet supported by this protoc plugin.
			-- sfixed64 = 2,

			floats = { 1.25, 2.5 },

			mapWithSetup = { [1] = 1, [2] = 4, [3] = 9 },
			mapWithEnums = { [1] = "One", [2] = "Two" },

			aliasedEnum = "AliasedEnum_One",

			NotCamelCase = 11,

			strings = { "a", "b", "c" },
			bools = { true, false, true },

			customJson = "foo",

			bytes = buffer.fromstring("hello"),
		})

		local encoded = kitchenSink:encode()
		local decoded = kitchen_sink.KitchenSink.decode(encoded)

		assertEquals(decoded, kitchenSink)
	end)
end)

it("should accept fields passed to new", function()
	local kitchenSink = kitchen_sink.KitchenSink.new({
		int32 = 100,
	})

	assertEquals(kitchenSink.int32, 100)
	assertEquals(kitchenSink.float, 0)
end)

tests.finish()
