local tests = require("./tests")

local kitchen_sink = require("./samples/kitchen_sink")

local assertEquals = tests.assertEquals
local describe = tests.describe
local it = tests.it

describe("json deserializing", function()
	it("should deserialize primitives", function()
		local kitchenSink = kitchen_sink.KitchenSink.jsonDecode({
			int32 = 1,
		})

		assertEquals(kitchenSink.int32, 1)
	end)
end)

tests.finish()
