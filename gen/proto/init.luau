local base64 = require("./base64")

local proto = {}

export type Message<T> = {
	new: () -> T,

	encode: (T) -> buffer,
	decode: (buffer) -> T,

	jsonEncode: (T) -> any,
	jsonDecode: ({ [string]: any }) -> T,
}

export type Enum<T> = {
	fromName: (string) -> T?,

	fromNumber: (number) -> T?,
	toNumber: (T) -> number,
}

-- TODO: Round-trip every number -2^31 through 2^31
function proto.readVarInt(input: buffer, cursor: number): (number, number)
	local width = 0
	repeat
		local b = buffer.readu8(input, cursor + width)
		width += 1
	until not bit32.btest(b, 0b1000_0000)

	local mask = 0b0111_1111

	-- The max 64-bit int (2^63-1) encodes into fewer than 10 bytes (9)
	-- and negatives always use 10 bytes, so we can branch safely
	if width < 10 then
		local final = 0

		for offset = 0, width - 1 do
			local byte = buffer.readu8(input, cursor + offset)
			final += (bit32.lshift(1, 7) ^ offset) * bit32.band(byte, mask)
		end

		return final, cursor + width
	end

	local final = bit32.band(buffer.readu8(input, cursor + 0), mask)
		+ bit32.band(buffer.readu8(input, cursor + 1), mask) * bit32.lshift(1, 7)
		+ bit32.band(buffer.readu8(input, cursor + 2), mask) * bit32.lshift(1, 7) ^ 2
		+ bit32.band(buffer.readu8(input, cursor + 3), mask) * bit32.lshift(1, 7) ^ 3
		+ bit32.band(buffer.readu8(input, cursor + 4), mask) * bit32.lshift(1, 7) ^ 4
		+ bit32.band(buffer.readu8(input, cursor + 5), mask) * bit32.lshift(1, 7) ^ 5
		+ bit32.band(buffer.readu8(input, cursor + 6), mask) * bit32.lshift(1, 7) ^ 6
		+ bit32.band(buffer.readu8(input, cursor + 7), 0b0000_1111) * bit32.lshift(1, 7) ^ 7

	return final - 2 ^ 53, cursor + width
end

function proto.readFloat(input: buffer, cursor: number): (number, number)
	return buffer.readf32(input, cursor), cursor + 4
end

function proto.readDouble(input: buffer, cursor: number): (number, number)
	return buffer.readf64(input, cursor), cursor + 8
end

function proto.readFixed32(input: buffer, cursor: number): (number, number)
	return buffer.readu32(input, cursor), cursor + 4
end

function proto.readSignedFixed32(input: buffer, cursor: number): (number, number)
	return buffer.readi32(input, cursor), cursor + 4
end

-- TODO: Test with 0 to 2^32
function proto.readFixed64(output: buffer, cursor: number): (number, number)
	-- =(
	local hex = string.format("%08x", buffer.readu32(output, cursor + 4))
		.. string.format("%08x", buffer.readu32(output, cursor))

	return assert(tonumber(hex, 16), "Couldn't parse created hex"), cursor + 8
end

function proto.readSignedFixed64(_output: buffer, _cursor: number): (number, number)
	error("TODO: Read sfixed64")
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
	local highestBitMask = bit32.lshift(1, 7)
	local lowerBitsMask = 0b0111_1111

	if number >= 0 then
		repeat
			output = reserveBuffer(output, cursor, 1)
			local nextByte = number % highestBitMask
			number = number // highestBitMask
			buffer.writeu8(output, cursor, if number == 0 then nextByte else nextByte + highestBitMask)
			cursor += 1
		until number == 0

		return output, cursor
	else
		local comp = 2 ^ 53 + number -- precision lost, could error

		output = reserveBuffer(output, cursor, 10)

		buffer.writeu8(output, cursor + 0, bit32.bor(highestBitMask, comp % highestBitMask))
		buffer.writeu8(output, cursor + 1, bit32.bor(highestBitMask, comp // highestBitMask % highestBitMask))
		buffer.writeu8(output, cursor + 2, bit32.bor(highestBitMask, comp // (highestBitMask ^ 2) % highestBitMask))
		buffer.writeu8(
			output,
			cursor + 3,
			bit32.bor(highestBitMask, bit32.band(comp // (highestBitMask ^ 3), lowerBitsMask))
		)
		buffer.writeu8(
			output,
			cursor + 4,
			bit32.bor(highestBitMask, bit32.band(comp // (highestBitMask ^ 4), lowerBitsMask))
		)
		buffer.writeu8(
			output,
			cursor + 5,
			bit32.bor(highestBitMask, bit32.band(comp // (highestBitMask ^ 5), lowerBitsMask))
		)
		buffer.writeu8(
			output,
			cursor + 6,
			bit32.bor(highestBitMask, bit32.band(comp // (highestBitMask ^ 6), lowerBitsMask))
		)
		buffer.writeu8(output, cursor + 7, bit32.bor(0b1_1110000, comp // (highestBitMask ^ 7)))
		buffer.writeu8(output, cursor + 8, 0b1111_1111)
		buffer.writeu8(output, cursor + 9, 0b0000_0001)

		return output, cursor + 10
	end
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

function proto.writeFixed32(output: buffer, cursor: number, input: number): (buffer, number)
	output = reserveBuffer(output, cursor, 4)
	buffer.writeu32(output, cursor, input)
	return output, cursor + 4
end

function proto.writeSignedFixed32(output: buffer, cursor: number, input: number): (buffer, number)
	output = reserveBuffer(output, cursor, 4)
	buffer.writei32(output, cursor, input)
	return output, cursor + 4
end

function proto.writeFixed64(output: buffer, cursor: number, input: number): (buffer, number)
	output = reserveBuffer(output, cursor, 8)

	-- =(
	local formatted = string.format("%016x", input)
	buffer.writeu32(output, cursor, (assert(tonumber(formatted:sub(9, 16), 16), "Couldn't parse created hex")))
	buffer.writeu32(output, cursor + 4, (assert(tonumber(formatted:sub(1, 8), 16), "Couldn't parse created hex")))

	return output, cursor + 8
end

function proto.writeSignedFixed64(_output: buffer, _cursor: number, _input: number): (buffer, number)
	error("TODO: Write sfixed64")
end

function proto.writeTag(output: buffer, cursor: number, field: number, wireType: number): (buffer, number)
	return proto.writeVarInt(output, cursor, bit32.lshift(field, 3) + wireType)
end

function proto.writeBuffer(output: buffer, cursor: number, input: buffer, length: number): (buffer, number)
	output, cursor = proto.writeVarInt(output, cursor, length)

	output = reserveBuffer(output, cursor, length)
	buffer.copy(output, cursor, input, 0, length)

	return output, cursor + length
end

function proto.writeString(output: buffer, cursor: number, text: string): (buffer, number)
	return proto.writeBuffer(output, cursor, buffer.fromstring(text), #text)
end

function proto.decodeZigZag(number: number): number
	return (if number % 2 == 1 then -number // 2 else number // 2)
end

function proto.encodeZigZag(number: number): number
	return if number < 0 then -2 * number - 1 else 2 * number
end

function proto.limitInt32(number: number): number
	local overflow = math.sign(number) * (math.abs(number) % (2 ^ 32))
	if overflow >= 2 ^ 31 then
		overflow -= 2 ^ 31
	elseif overflow <= -2 ^ 31 then
		overflow += 2 ^ 31
	end

	return overflow
end

proto.json = {}

function proto.json.serializeNumber(input: number): string | number
	return if input == math.huge
		then "Infinity"
		elseif input == -math.huge then "-Infinity"
		elseif input ~= input then "NaN"
		else input
end

function proto.json.deserializeNumber(input: string | number): number
	if input == "Infinity" then
		return math.huge
	elseif input == "-Infinity" then
		return -math.huge
	elseif input == "NaN" then
		return 0 / 0
	elseif typeof(input) == "string" then
		local number = tonumber(input)
		assert(number ~= nil, "Number given that could not be parsed")
		return number
	else
		return input
	end
end

function proto.json.serializeBuffer(input: buffer): string
	return base64.encode(buffer.tostring(input))
end

function proto.json.deserializeBuffer(input: string): buffer
	return buffer.fromstring(base64.decode(input))
end

return proto
