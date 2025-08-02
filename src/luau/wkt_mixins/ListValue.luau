function _ListValueImpl.jsonEncode(list: ListValue): { any }
	local serialized = {}

	for _, value in list.values do
		table.insert(serialized, messages.Value.jsonEncode(value))
	end

	return serialized
end

function _ListValueImpl.jsonDecode(anyValue: any): ListValue
	local input: { any } = anyValue

	local values = {}

	for _, value in input do
		table.insert(values, messages.Value.jsonDecode(value))
	end

	return _ListValueImpl.new({
		values = values,
	})
end
