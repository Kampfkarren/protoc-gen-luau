--!strict
local proto = require("./")

local function assertEquals<T>(x: T, y: T)
	if x ~= y then
		error(`{x} ~= {y}`, 2)
	end
end

local function formatHex(input: string): string
	local bytes = {}

	for index = 1, #input do
		table.insert(bytes, string.format("%02x", string.byte(input:sub(index, index + 1))))
	end

	return table.concat(bytes, " ")
end

local function assertBufferEquals(input: buffer, bufferLength: number, expected: string)
	local shrunkBuffer = buffer.create(bufferLength)
	buffer.copy(shrunkBuffer, 0, input, 0, bufferLength)

	local bufferString = buffer.tostring(shrunkBuffer)

	if bufferString ~= expected then
		error(`Buffers don't match\n{formatHex(bufferString)}\n{formatHex(expected)}`, 2)
	end
end

do
	local number, newCursor = proto.readVarInt(buffer.fromstring("\x96\x01"), 0)
	assertEquals(number, 150)
	assertEquals(newCursor, 2)
end

do
	local output, newCursor = proto.writeVarInt(buffer.create(0), 0, 150)

	assertEquals(newCursor, 2)
	assertBufferEquals(output, newCursor, "\x96\x01")
end

do
	local index, wireType, newCursor = proto.readTag(buffer.fromstring("\x08"), 0)
	assertEquals(index, 1)
	assertEquals(wireType, 0)
	assertEquals(newCursor, 1)
end

do
	local output, newCursor = proto.writeTag(buffer.create(0), 0, 1, proto.wireTypes.varint)
	assertBufferEquals(output, newCursor, "\x08")
	assertEquals(newCursor, 1)
end

do
	local number, newCursor = proto.readVarInt(buffer.fromstring("\x9C\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x01"), 0)
	assertEquals(number, -100)
	assertEquals(newCursor, 10)
end

do
	local number, newCursor = proto.readVarInt(buffer.fromstring("\x80\x80\x80\x80\xf8\xff\xff\xff\xff\x01"), 0)
	assertEquals(number, -2 ^ 31)
	assertEquals(newCursor, 10)
end

do
	local output, newCursor = proto.writeString(buffer.create(0), 0, "testing")
	assertBufferEquals(output, newCursor, "\x07testing")
	assertEquals(newCursor, 8)
end
