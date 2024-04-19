--!strict
local tests = require("./tests")

local kitchen_sink = require("./samples/kitchen_sink")
local recursive = require("./samples/recursive")
local type_registry = require("./samples/proto/typeRegistry")

local assertEquals = tests.assertEquals
local assertNotEquals = tests.assertNotEquals
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

describe("type registry", function()
	describe("created ad hoc", function()
		it("should find messages by name", function()
			local registry = type_registry.TypeRegistry.new({
				kitchen_sink.KitchenSink,
			})

			assertNotEquals(registry:findMessage("my.package.KitchenSink"), nil)

			-- Timestamp is not in the registry.
			assertEquals(registry:findMessage("google.protobuf.Timestamp"), nil)
		end)
	end)

	describe("default registry", function()
		it("should find messages by name", function()
			assertNotEquals(type_registry.default:findMessage("my.package.KitchenSink"), nil)
		end)
	end)

end)

tests.finish()
