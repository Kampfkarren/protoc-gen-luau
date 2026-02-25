--!strict
-- Validates that generated protos use camelCase field names (field_name_case=camel).
-- This file is run by the Rust test after generating with field_name_case=camel into samples/field_case_camel/.
local tests = require("./tests")
local field_case_test = require("./samples/field_case_test")
local assertEquals = tests.assertEquals
local describe = tests.describe
local it = tests.it

describe("field_name_case camel", function()
	it("should accept and expose fields via camelCase", function()
		local msg = field_case_test.FieldCaseTest.new({
			stringValue = "hello",
			otherField = 42,
			nonSnakeCaseField = true,
		})
		assertEquals(msg.stringValue, "hello")
		assertEquals(msg.otherField, 42)
		assertEquals(msg.nonSnakeCaseField, true)
	end)

	it("should round-trip encode and decode with camelCase fields", function()
		local msg = field_case_test.FieldCaseTest.new({
			stringValue = "test",
			otherField = 1,
			nonSnakeCaseField = false,
		})
		local encoded = msg:encode()
		local decoded = field_case_test.FieldCaseTest.decode(encoded)
		assertEquals(decoded.stringValue, "test")
		assertEquals(decoded.otherField, 1)
		assertEquals(decoded.nonSnakeCaseField, false)
	end)
end)

tests.finish()
