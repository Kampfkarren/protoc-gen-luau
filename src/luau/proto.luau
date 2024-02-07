local base64 = require("./base64")

local proto = {}

export type Message<T> = {
	new: () -> T,

	encode: (T) -> buffer,
	decode: (buffer) -> T,

	jsonEncode: (T) -> any,
	jsonDecode: (any) -> T,
}

export type Enum<T> = {
	fromNumber: (number) -> T?,
	toNumber: (T) -> number,
}

-- TODO: Round-trip every number 1 through 2^31
function proto.readVarInt(input: buffer, cursor: number): (number, number)
	local numberBytes = 0
	local finalNumber = 0

	repeat
		local byte = buffer.readu8(input, cursor + numberBytes)
		local lowerBits = bit32.band(byte, 0b0111_1111)

		finalNumber += bit32.lshift(lowerBits, numberBytes * 7)
		numberBytes += 1
	until bit32.rshift(byte, 7) == 0

	-- TODO: Error with 64-bit integers
	if numberBytes > 4 then
		finalNumber = bit32.band(finalNumber, 0x7FFFFFFF) - 2 ^ 31
	end

	return finalNumber, cursor + numberBytes
end

function proto.readFloat(input: buffer, cursor: number): (number, number)
	return buffer.readf32(input, cursor), cursor + 4
end

function proto.readDouble(input: buffer, cursor: number): (number, number)
	return buffer.readf64(input, cursor), cursor + 8
end

function proto.readTag(input: buffer, cursor: number): (number, number, number)
	local varInt, newCursor = proto.readVarInt(input, cursor)
	return bit32.rshift(varInt, 3), bit32.band(varInt, 0b111), newCursor
end

function proto.readBuffer(input: buffer, cursor: number): (buffer, number)
	local length, newCursor = proto.readVarInt(input, cursor)
	local output = buffer.create(length)
	buffer.copy(output, 0, input, newCursor, length)
	return output, newCursor + length
end

local function reserveBuffer(output: buffer, cursor: number, amount: number): buffer
	if buffer.len(output) - cursor < amount then
		local newBuffer = buffer.create(3 ^ math.ceil(math.log(buffer.len(output) + amount, 3)))
		buffer.copy(newBuffer, 0, output, 0, cursor)
		return newBuffer
	else
		return output
	end
end

proto.wireTypes = {
	varint = 0,
	i64 = 1,
	lengthDelimited = 2,
	sgroup = 3,
	egroup = 4,
	i32 = 5,
}

-- TODO: Test negative number
function proto.writeVarInt(output: buffer, cursor: number, number: number): (buffer, number)
	assert(number >= 0, "NYI: Negative varints")
	-- TODO: Assert against non-32 bit ints
	-- TODO: Assert against doubles

	local bytesWritten = 0

	repeat
		local nextSegment = bit32.band(number, 0b0111_1111)
		number = bit32.rshift(number, 7)

		if number > 0 then
			nextSegment += 0b1000_0000
		end

		output = reserveBuffer(output, cursor + bytesWritten, 1)
		buffer.writeu8(output, cursor + bytesWritten, nextSegment)

		bytesWritten += 1
	until number <= 0

	return output, cursor + bytesWritten
end

function proto.writeFloat(output: buffer, cursor: number, float: number): (buffer, number)
	output = reserveBuffer(output, cursor, 4)
	buffer.writef32(output, cursor, float)
	return output, cursor + 4
end

function proto.writeDouble(output: buffer, cursor: number, double: number): (buffer, number)
	output = reserveBuffer(output, cursor, 8)
	buffer.writef64(output, cursor, double)
	return output, cursor + 8
end

function proto.writeTag(output: buffer, cursor: number, field: number, wireType: number): (buffer, number)
	return proto.writeVarInt(output, cursor, bit32.lshift(field, 3) + wireType)
end

function proto.writeBuffer(output: buffer, cursor: number, input: buffer): (buffer, number)
	local bufferLength = buffer.len(input)
	output, cursor = proto.writeVarInt(output, cursor, bufferLength)

	output = reserveBuffer(output, cursor, bufferLength)
	buffer.copy(output, cursor, input, 0, bufferLength)

	return output, cursor + bufferLength
end

function proto.writeString(output: buffer, cursor: number, text: string): (buffer, number)
	return proto.writeBuffer(output, cursor, buffer.fromstring(text))
end

proto.json = {}

-- selene: allow(undefined_variable)
function proto.json.jsonEncode(input: any): string
	if game == nil then
		return game:GetService("HttpService"):JSONEncode(input)
	elseif string.match(_VERSION, "^Lune") ~= nil then
		return require("@lune/net").jsonEncode(input)
	else
		error("No JSON encoding available")
	end
end

-- selene: allow(undefined_variable)
function proto.json.jsonDecode(input: string): any
	if game == nil then
		return game:GetService("HttpService"):JSONDecode(input)
	elseif string.match(_VERSION, "^Lune") ~= nil then
		return require("@lune/net").jsonDecode(input)
	else
		error("No JSON decoding available")
	end
end

function proto.json.serializeNumber(input: number): string | number
	return if input == math.huge
		then "Infinity"
		elseif input == -math.huge then "-Infinity"
		elseif input ~= input then "NaN"
		else input
end

function proto.json.serializeBuffer(input: buffer): string
	return base64.decode(buffer.tostring(input))
end

return proto
