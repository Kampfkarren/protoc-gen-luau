function _StructImpl.jsonEncode(struct: Struct): { [string]: any }
	local serialized = {}

	for key, value in struct.fields do
		serialized[key] = messages.Value.jsonEncode(value)
	end

	return serialized
end

function _StructImpl.jsonDecode(input: { [string]: any }): Struct
	local fields = {}

	for key, serializedValue in input do
		fields[key] = messages.Value.jsonDecode(serializedValue)
	end

	return Struct.new({
		fields = fields,
	})
end
