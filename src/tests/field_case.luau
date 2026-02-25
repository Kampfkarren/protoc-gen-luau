--!strict
-- Validates that generated protos use snake_case field names (default / field_name_case=snake).
local tests = require("./tests")
local field_case_test = require("./samples/field_case_test")
local assertEquals = tests.assertEquals
local describe = tests.describe
local it = tests.it

describe("field_name_case snake (default)", function()
	it("should accept and expose fields via snake_case", function()
		local msg = field_case_test.FieldCaseTest.new({
			string_value = "hello",
			other_field = 42,
		})
		assertEquals(msg.string_value, "hello")
		assertEquals(msg.other_field, 42)
	end)

	it("should round-trip encode and decode with snake_case fields", function()
		local msg = field_case_test.FieldCaseTest.new({
			string_value = "test",
			other_field = 1,
		})
		local encoded = msg:encode()
		local decoded = field_case_test.FieldCaseTest.decode(encoded)
		assertEquals(decoded.string_value, "test")
		assertEquals(decoded.other_field, 1)
	end)
end)

tests.finish()
