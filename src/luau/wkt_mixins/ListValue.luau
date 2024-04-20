function _ListValueImpl.jsonEncode(list: ListValue): { any }
	local serialized = {}

	for _, value in list.values do
		table.insert(serialized, Value.jsonEncode(value))
	end

	return serialized
end

function _ListValueImpl.jsonDecode(anyValue: any): ListValue
	local input: { any } = anyValue

	local values = {}

	for _, value in input do
		table.insert(values, Value.jsonDecode(value))
	end

	return ListValue.new({
		values = values,
	})
end
