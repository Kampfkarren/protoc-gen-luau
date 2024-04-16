--!strict
local tests = require("./tests")

local kitchen_sink = require("./samples/kitchen_sink")
local recursive = require("./samples/recursive")

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

		assertEquals(decoded.b.a.number, 100)
	end)
end)

describe("descriptors can return", function()
	it("full message name", function()
		assertEquals(kitchen_sink.KitchenSink.new():descriptor().full_name(), "my.package.KitchenSink")
	end)

	it("unqualified message name", function()
		assertEquals(kitchen_sink.KitchenSink.new():descriptor().name(), "KitchenSink")
	end)

end)

tests.finish()
