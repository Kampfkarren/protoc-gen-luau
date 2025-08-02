function _ValueImpl.jsonEncode(input: Value): any
	local kind = input.kind

	if kind == nil or kind.type == "null_value" then
		return nil
	elseif kind.type == "number_value" or kind.type == "string_value" or kind.type == "bool_value" then
		return kind.value
	elseif kind.type == "list_value" then
		return messages.ListValue.jsonEncode(kind.value)
	elseif kind.type == "struct_value" then
		return messages.Struct.jsonEncode(kind.value)
	else
		local exhaustiveMatch: never = kind.type
		return exhaustiveMatch
	end
end

function _ValueImpl.jsonDecode(input: any): Value
	if input == nil then
		return _ValueImpl.new({ kind = { type = "null_value", value = "NULL_VALUE" } })
	elseif typeof(input) == "number" then
		return _ValueImpl.new({ kind = { type = "number_value", value = input } })
	elseif typeof(input) == "string" then
		return _ValueImpl.new({ kind = { type = "string_value", value = input } })
	elseif typeof(input) == "boolean" then
		return _ValueImpl.new({ kind = { type = "bool_value", value = input } })
	elseif typeof(input) == "table" then
		local isArray = true
		local count = 0
		for _ in input do
			count += 1
			if count > #input then
				isArray = false
				break
			end
		end

		isArray = count == #input

		if isArray then
			return _ValueImpl.new({ kind = { type = "list_value", value = messages.ListValue.jsonDecode(input) } })
		else
			return _ValueImpl.new({ kind = { type = "struct_value", value = messages.Struct.jsonDecode(input) } })
		end
	else
		error(`Unknown input type: {typeof(input)}`)
	end
end
