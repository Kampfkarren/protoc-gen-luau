--!strict
-- Validates that .new() auto-wraps plain-table nested message fields into
-- typed instances, and passes already-typed instances through unchanged.
local tests = require("./tests")
local nested_autowrap = require("./samples/nested_autowrap")

local assertEquals = tests.assertEquals
local describe = tests.describe
local it = tests.it

local Container = nested_autowrap.Container
local Inner = nested_autowrap.Inner
local Other = nested_autowrap.Other

describe("nested_autowrap", function()
	describe("singular message field", function()
		it("wraps a plain table", function()
			local c = Container.new({
				single = { value = "hello" },
			})
			assertEquals(c.single.value, "hello")
			assert(getmetatable(c.single :: any) ~= nil, "expected c.single to have a metatable")
		end)

		it("passes a typed instance through unchanged", function()
			local inner = Inner.new({ value = "hello" })
			local c = Container.new({ single = inner })
			assertEquals(c.single, inner)
		end)
	end)

	describe("repeated message field", function()
		it("wraps each plain-table element", function()
			local c = Container.new({
				items = { { value = "a" }, { value = "b" } },
			})
			assertEquals(#c.items, 2)
			assertEquals(c.items[1].value, "a")
			assertEquals(c.items[2].value, "b")
			assert(getmetatable(c.items[1] :: any) ~= nil, "expected items[1] to have a metatable")
			assert(getmetatable(c.items[2] :: any) ~= nil, "expected items[2] to have a metatable")
		end)

		it("passes typed-instance elements through", function()
			local a = Inner.new({ value = "a" })
			local b = Inner.new({ value = "b" })
			local c = Container.new({ items = { a, b } })
			assertEquals(c.items[1], a)
			assertEquals(c.items[2], b)
		end)
	end)

	describe("map<string, message>", function()
		it("wraps each plain-table value", function()
			local c = Container.new({
				entries = { foo = { value = "x" }, bar = { value = "y" } },
			})
			assertEquals(c.entries.foo.value, "x")
			assertEquals(c.entries.bar.value, "y")
			assert(getmetatable(c.entries.foo :: any) ~= nil, "expected entries.foo to have a metatable")
			assert(getmetatable(c.entries.bar :: any) ~= nil, "expected entries.bar to have a metatable")
		end)

		it("passes typed-instance values through", function()
			local foo = Inner.new({ value = "x" })
			local c = Container.new({ entries = { foo = foo } })
			assertEquals(c.entries.foo, foo)
		end)
	end)

	describe("oneof with message case", function()
		it("wraps the plain-table value of a message case", function()
			local c = Container.new({
				payload = { type = "case_msg", value = { value = "z" } },
			})
			assertEquals((c.payload :: any).type, "case_msg")
			assertEquals((c.payload :: any).value.value, "z")
			assert(
				getmetatable((c.payload :: any).value) ~= nil,
				"expected payload.value to have a metatable"
			)
		end)

		it("wraps a second message-typed case too", function()
			local c = Container.new({
				payload = { type = "case_other", value = { n = 42 } },
			})
			assertEquals((c.payload :: any).value.n, 42)
			assert(
				getmetatable((c.payload :: any).value) ~= nil,
				"expected payload.value to have a metatable"
			)
		end)

		it("passes typed-instance value through", function()
			local inner = Inner.new({ value = "z" })
			local c = Container.new({
				payload = { type = "case_msg", value = inner },
			})
			assertEquals((c.payload :: any).value, inner)
		end)

		it("leaves non-message oneof variants (string) untouched", function()
			local c = Container.new({
				payload = { type = "case_string", value = "plain" },
			})
			assertEquals((c.payload :: any).type, "case_string")
			assertEquals((c.payload :: any).value, "plain")
		end)
	end)

	describe("round-trip through encode/decode", function()
		it("jsonEncode + jsonDecode works on plain-input construction", function()
			local c = Container.new({
				single = { value = "s" },
				items = { { value = "a" }, { value = "b" } },
				entries = { k = { value = "v" } },
				payload = { type = "case_msg", value = { value = "p" } },
			})

			local json = c:jsonEncode()
			local decoded = Container.jsonDecode(json)

			assert(decoded.single ~= nil, "expected decoded.single")
			assertEquals((decoded.single :: any).value, "s")
			assertEquals(#decoded.items, 2)
			assertEquals((decoded.items[1] :: any).value, "a")
			assertEquals((decoded.entries.k :: any).value, "v")
			assertEquals((decoded.payload :: any).type, "case_msg")
			assertEquals((decoded.payload :: any).value.value, "p")
		end)

		it("binary encode + decode works on plain-input construction", function()
			local c = Container.new({
				single = { value = "s" },
				items = { { value = "a" } },
				entries = { k = { value = "v" } },
				payload = { type = "case_other", value = { n = 7 } },
			})

			local buf = c:encode()
			local decoded = Container.decode(buf)

			assertEquals((decoded.single :: any).value, "s")
			assertEquals((decoded.items[1] :: any).value, "a")
			assertEquals((decoded.entries.k :: any).value, "v")
			assertEquals((decoded.payload :: any).type, "case_other")
			assertEquals((decoded.payload :: any).value.n, 7)
		end)
	end)
end)

tests.finish()
