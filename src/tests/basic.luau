local tests = require("./tests")

local kitchen_sink = require("./samples/kitchen_sink")

local assertEquals = tests.assertEquals
local assertTablesEquivalent = tests.assertTablesEquivalent
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

		assertTablesEquivalent(kitchenSink:jsonEncode(), json)
	end)
end)

tests.finish()
