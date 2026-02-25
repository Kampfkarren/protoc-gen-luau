--!strict
-- Validates that generated protos preserve field names as-is when no field_name_case option is passed.
local tests = require("./tests")
local field_case_test = require("./samples/field_case_test/field_case_test")
local assertEquals = tests.assertEquals
local describe = tests.describe
local it = tests.it

describe("field_name_case preserve (default)", function()
	it("should accept and expose fields with original names", function()
		local msg = field_case_test.FieldCaseTest.new({
			string_value = "hello",
			other_field = 42,
			NON_SNAKE_CASE_FIELD = true,
		})
		assertEquals(msg.string_value, "hello")
		assertEquals(msg.other_field, 42)
		assertEquals(msg.NON_SNAKE_CASE_FIELD, true)
	end)

	it("should round-trip encode and decode with preserved field names", function()
		local msg = field_case_test.FieldCaseTest.new({
			string_value = "test",
			other_field = 1,
			NON_SNAKE_CASE_FIELD = false,
		})
		local encoded = msg:encode()
		local decoded = field_case_test.FieldCaseTest.decode(encoded)
		assertEquals(decoded.string_value, "test")
		assertEquals(decoded.other_field, 1)
		assertEquals(decoded.NON_SNAKE_CASE_FIELD, false)
	end)
end)

tests.finish()
