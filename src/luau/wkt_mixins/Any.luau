function _AnyImpl.jsonEncode(input: Any): { [string]: any }
	local unpacked = input:unpack(typeRegistry.default)
	assert(unpacked ~= nil, "Cannot JSON-encode empty Any")

	local json = unpacked:jsonEncode()

	if typeof(json) == "string" then
		return { ["@type"] = input.type_url, ["value"] = json }
	end

	json["@type"] = input.type_url

	return json
end

function _AnyImpl.jsonDecode(input: { [string]: any }): Any
	local typeUrl = input["@type"]
	local typeName = typeUrl:match("([^/]+)$")
	local payloadType = typeRegistry.default:findMessage(typeName)

	if payloadType == nil then
		error(`Could not find message type \`{typeName}\` for Any`)
	end

	local typePrefix = typeUrl:sub(1, #typeUrl - #typeName - 1)

	local json = table.clone(input)
	-- For WKTs with special JSON representations, we need to decode input["value"] rather than
	-- input.
	if typeName:match("^google%.protobuf%.") and input["value"] ~= nil then
		json = input["value"]
	else
		-- Clear this because it's not part of the JSON encoding of the payload itself.
		json["@type"] = nil
	end

	return _AnyImpl.pack(payloadType.jsonDecode(json), typePrefix)
end
