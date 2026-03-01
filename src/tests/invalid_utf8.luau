--!strict
--# selene: allow(bad_string_escape)
local tests = require("./tests")
local kitchen_sink = require("./samples/kitchen_sink")

local assertEquals = tests.assertEquals
local describe = tests.describe
local it = tests.it

local UNICODE_REPLACEMENT_CHARACTER = "\xEF\xBF\xBD"

-- Helper to build a raw protobuf binary with a single string field.
-- Uses field 33 (text) of KitchenSink, which is a `string` field.
-- Tag: (33 << 3) | 2 = 266 -> varint bytes 0x8A, 0x02
local function makeRawStringMessage(bytes: { number }): buffer
	local raw = buffer.create(2 + 1 + #bytes)
	buffer.writeu8(raw, 0, 0x8A) -- tag byte 1
	buffer.writeu8(raw, 1, 0x02) -- tag byte 2
	buffer.writeu8(raw, 2, #bytes) -- length (assumes < 128)
	for i, byte in bytes do
		buffer.writeu8(raw, 2 + i, byte)
	end
	return raw
end

describe("invalid UTF-8 handling", function()
	describe("decoding binary with invalid UTF-8 in string fields", function()
		it("should replace a lone continuation byte (0x80) with U+FFFD", function()
			local raw = makeRawStringMessage({ 0x80 })
			local decoded = kitchen_sink.KitchenSink.decode(raw)
			assertEquals(decoded.text, UNICODE_REPLACEMENT_CHARACTER)
		end)

		it("should replace byte 0xFF with U+FFFD", function()
			local raw = makeRawStringMessage({ 0xFF })
			local decoded = kitchen_sink.KitchenSink.decode(raw)
			assertEquals(decoded.text, UNICODE_REPLACEMENT_CHARACTER)
		end)

		it("should replace byte 0xFE with U+FFFD", function()
			local raw = makeRawStringMessage({ 0xFE })
			local decoded = kitchen_sink.KitchenSink.decode(raw)
			assertEquals(decoded.text, UNICODE_REPLACEMENT_CHARACTER)
		end)

		it("should replace a truncated 2-byte sequence with U+FFFD", function()
			-- 0xC3 is the start of a 2-byte sequence but has no continuation byte
			local raw = makeRawStringMessage({ 0xC3 })
			local decoded = kitchen_sink.KitchenSink.decode(raw)
			assertEquals(decoded.text, UNICODE_REPLACEMENT_CHARACTER)
		end)

		it("should replace only the invalid byte in otherwise valid ASCII", function()
			-- "A" 0x80 "B" - continuation byte embedded in otherwise valid ASCII
			local raw = makeRawStringMessage({ 0x41, 0x80, 0x42 })
			local decoded = kitchen_sink.KitchenSink.decode(raw)
			assertEquals(decoded.text, "A" .. UNICODE_REPLACEMENT_CHARACTER .. "B")
		end)
	end)

	describe("valid UTF-8 should still decode correctly", function()
		it("should decode plain ASCII", function()
			local raw = makeRawStringMessage({ 0x68, 0x65, 0x6C, 0x6C, 0x6F }) -- "hello"
			local decoded = kitchen_sink.KitchenSink.decode(raw)
			assertEquals(decoded.text, "hello")
		end)

		it("should decode multi-byte characters (e-acute)", function()
			-- "héllo" = 0x68, 0xC3, 0xA9, 0x6C, 0x6C, 0x6F
			local raw = makeRawStringMessage({ 0x68, 0xC3, 0xA9, 0x6C, 0x6C, 0x6F })
			local decoded = kitchen_sink.KitchenSink.decode(raw)
			assertEquals(decoded.text, "h\xC3\xA9llo")
		end)

		it("should decode 3-byte characters (CJK)", function()
			-- U+65E5 (日) = 0xE6 0x97 0xA5
			local raw = makeRawStringMessage({ 0xE6, 0x97, 0xA5 })
			local decoded = kitchen_sink.KitchenSink.decode(raw)
			assertEquals(decoded.text, "\xE6\x97\xA5")
		end)

		it("should decode 4-byte characters (emoji)", function()
			-- U+1F600 (😀) = 0xF0 0x9F 0x98 0x80
			local raw = makeRawStringMessage({ 0xF0, 0x9F, 0x98, 0x80 })
			local decoded = kitchen_sink.KitchenSink.decode(raw)
			assertEquals(decoded.text, "\xF0\x9F\x98\x80")
		end)

		it("should round-trip valid multi-byte strings through encode/decode", function()
			local original = kitchen_sink.KitchenSink.new({
				text = "\xE6\x97\xA5\xE6\x9C\xAC\xE8\xAA\x9E", -- 日本語
			})
			local encoded = original:encode()
			local decoded = kitchen_sink.KitchenSink.decode(encoded)
			assertEquals(decoded.text, original.text)
		end)

		it("should handle the empty string", function()
			local raw = makeRawStringMessage({})
			local decoded = kitchen_sink.KitchenSink.decode(raw)
			assertEquals(decoded.text, "")
		end)
	end)

	describe("encoding should replace invalid UTF-8 with U+FFFD", function()
		it("should replace a lone continuation byte during encode", function()
			local msg = kitchen_sink.KitchenSink.new({
				text = "\x80",
			})
			local encoded = msg:encode()
			local decoded = kitchen_sink.KitchenSink.decode(encoded)
			assertEquals(decoded.text, UNICODE_REPLACEMENT_CHARACTER)
		end)

		it("should replace 0xFF during encode", function()
			local msg = kitchen_sink.KitchenSink.new({
				text = "\xFF",
			})
			local encoded = msg:encode()
			local decoded = kitchen_sink.KitchenSink.decode(encoded)
			assertEquals(decoded.text, UNICODE_REPLACEMENT_CHARACTER)
		end)

		it("should replace a truncated multi-byte sequence during encode", function()
			local msg = kitchen_sink.KitchenSink.new({
				text = "\xC3", -- start of 2-byte sequence, missing continuation
			})
			local encoded = msg:encode()
			local decoded = kitchen_sink.KitchenSink.decode(encoded)
			assertEquals(decoded.text, UNICODE_REPLACEMENT_CHARACTER)
		end)

		it("should replace invalid bytes embedded within valid UTF-8 during encode", function()
			local msg = kitchen_sink.KitchenSink.new({
				text = "hello\x80world",
			})
			local encoded = msg:encode()
			local decoded = kitchen_sink.KitchenSink.decode(encoded)
			assertEquals(decoded.text, "hello" .. UNICODE_REPLACEMENT_CHARACTER .. "world")
		end)

		it("should replace invalid bytes at the start of a string during encode", function()
			local msg = kitchen_sink.KitchenSink.new({
				text = "\xFFhello",
			})
			local encoded = msg:encode()
			local decoded = kitchen_sink.KitchenSink.decode(encoded)
			assertEquals(decoded.text, UNICODE_REPLACEMENT_CHARACTER .. "hello")
		end)

		it("should replace invalid bytes at the end of a string during encode", function()
			local msg = kitchen_sink.KitchenSink.new({
				text = "hello\xFE",
			})
			local encoded = msg:encode()
			local decoded = kitchen_sink.KitchenSink.decode(encoded)
			assertEquals(decoded.text, "hello" .. UNICODE_REPLACEMENT_CHARACTER)
		end)

		it("should replace multiple invalid bytes scattered through a string during encode", function()
			local msg = kitchen_sink.KitchenSink.new({
				text = "one\x80two\xFFthree",
			})
			local encoded = msg:encode()
			local decoded = kitchen_sink.KitchenSink.decode(encoded)
			assertEquals(
				decoded.text,
				"one" .. UNICODE_REPLACEMENT_CHARACTER .. "two" .. UNICODE_REPLACEMENT_CHARACTER .. "three"
			)
		end)

		it("should replace invalid bytes at start, middle, and end during encode", function()
			local msg = kitchen_sink.KitchenSink.new({
				text = "\xFEhello\x80world\xFF",
			})
			local encoded = msg:encode()
			local decoded = kitchen_sink.KitchenSink.decode(encoded)
			assertEquals(
				decoded.text,
				UNICODE_REPLACEMENT_CHARACTER
					.. "hello"
					.. UNICODE_REPLACEMENT_CHARACTER
					.. "world"
					.. UNICODE_REPLACEMENT_CHARACTER
			)
		end)

		it("should replace invalid UTF-8 in a repeated string field", function()
			local msg = kitchen_sink.KitchenSink.new({
				strings = { "valid", "\x80\xFF", "also valid" },
			})
			local encoded = msg:encode()
			local decoded = kitchen_sink.KitchenSink.decode(encoded)
			assertEquals(
				decoded.strings,
				{ "valid", UNICODE_REPLACEMENT_CHARACTER .. UNICODE_REPLACEMENT_CHARACTER, "also valid" }
			)
		end)

		it("should replace invalid UTF-8 in a oneof string field", function()
			local msg = kitchen_sink.KitchenSink.new({
				oneofValue = { type = "y", value = "\xFE\xFF" },
			})
			local encoded = msg:encode()
			local decoded = kitchen_sink.KitchenSink.decode(encoded)
			assertEquals(
				decoded.oneofValue,
				{ type = "y", value = UNICODE_REPLACEMENT_CHARACTER .. UNICODE_REPLACEMENT_CHARACTER }
			)
		end)

		it("should replace invalid UTF-8 in a map string value", function()
			local msg = kitchen_sink.KitchenSink.new({
				map = { [true] = "\x80invalid" },
			})
			local encoded = msg:encode()
			local decoded = kitchen_sink.KitchenSink.decode(encoded)
			assertEquals(decoded.map, { [true] = UNICODE_REPLACEMENT_CHARACTER .. "invalid" })
		end)
	end)

	describe("bytes fields should accept arbitrary data", function()
		it("should round-trip invalid UTF-8 in a bytes field without error", function()
			local original = kitchen_sink.KitchenSink.new({
				bytes = buffer.fromstring("\x80\xFF\xFE\xC0"),
			})
			local encoded = original:encode()
			local decoded = kitchen_sink.KitchenSink.decode(encoded)
			assertEquals(buffer.tostring(decoded.bytes), "\x80\xFF\xFE\xC0")
		end)
	end)
end)

tests.finish()
