--!strict
--!nolint LocalUnused
--!nolint ImportUnused
--# selene: allow(empty_if, if_same_then_else, manual_table_clone, unused_variable)
-- This file was @autogenerated by protoc-gen-luau
local proto = require("../../proto")
local typeRegistry = require("../../proto/typeRegistry")

type _FieldMaskImpl = {
	__index: _FieldMaskImpl,
	new: (fields: _FieldMaskPartialFields?) -> FieldMask,
	encode: (self: FieldMask) -> buffer,
	decode: (input: buffer) -> FieldMask,
	jsonEncode: (self: FieldMask) -> string,
	jsonDecode: (input: string) -> FieldMask,
	descriptor: proto.Descriptor,
}

type _FieldMaskFields = {
	paths: { string },
}

type _FieldMaskPartialFields = {
	paths: { string }?,
}

export type FieldMask = typeof(setmetatable({} :: _FieldMaskFields, {} :: _FieldMaskImpl))
local FieldMask: proto.Message<FieldMask, _FieldMaskPartialFields> & proto.CustomJson<FieldMask, string>

local _FieldMaskImpl = {}
_FieldMaskImpl.__index = _FieldMaskImpl

function _FieldMaskImpl.new(data: _FieldMaskPartialFields?): FieldMask
	return setmetatable({
		paths = if data == nil or data.paths == nil then {} else data.paths,
	}, _FieldMaskImpl)
end

function _FieldMaskImpl.encode(self: FieldMask): buffer
	local output = buffer.create(0)
	local cursor = 0

	if self.paths ~= nil and #self.paths > 0 then
		for _, value in self.paths do
			output, cursor = proto.writeTag(output, cursor, 1, proto.wireTypes.lengthDelimited)
			output, cursor = proto.writeString(output, cursor, value)
		end
	end

	local shrunkBuffer = buffer.create(cursor)
	buffer.copy(shrunkBuffer, 0, output, 0, cursor)
	return shrunkBuffer
end

function _FieldMaskImpl.decode(input: buffer): FieldMask
	local self = _FieldMaskImpl.new()
	local cursor = 0

	while cursor < buffer.len(input) do
		local field, wireType
		field, wireType, cursor = proto.readTag(input, cursor)

		if wireType == proto.wireTypes.varint then
			-- No fields

			local _
			_, cursor = proto.readVarInt(input, cursor)
		elseif wireType == proto.wireTypes.lengthDelimited then
			if field == 1 then
				local value
				value, cursor = proto.readBuffer(input, cursor)
				table.insert(self.paths, buffer.tostring(value))
				continue
			end

			local length
			length, cursor = proto.readVarInt(input, cursor)

			cursor += length
		elseif wireType == proto.wireTypes.i32 then
			-- No fields

			local _
			_, cursor = proto.readFixed32(input, cursor)
		elseif wireType == proto.wireTypes.i64 then
			-- No fields

			local _
			_, cursor = proto.readFixed64(input, cursor)
		else
			error("Unsupported wire type: " .. wireType)
		end
	end

	return self
end

function _FieldMaskImpl.jsonEncode(fieldMask: FieldMask): string
	return table.concat(fieldMask.paths, ",")
end

function _FieldMaskImpl.jsonDecode(anyInput: any): FieldMask
	local input: string = anyInput

	local paths = {}
	for path in string.gmatch(input, "[^,]+") do
		table.insert(paths, path)
	end

	return FieldMask.new({
		paths = paths,
	})
end

_FieldMaskImpl.descriptor = {
	name = "FieldMask",
	fullName = "google.protobuf.FieldMask",
}

FieldMask = _FieldMaskImpl :: any -- Luau: Not sure why this intersection fails.

typeRegistry.default:register(FieldMask)

return {
	FieldMask = FieldMask,
}
