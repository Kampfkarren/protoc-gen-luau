function _AnyImpl.pack<T, F>(payload: proto.Message<T, F>, typeUrlPrefix: string): Any
	return Any.new({
		type_url = typeUrlPrefix .. "/" .. payload.descriptor.fullName,
		value = payload:encode(),
	})
end

function _AnyImpl.unpack<T, F>(self: Any, registry: typeRegistry.TypeRegistry?): proto.Message<T, F>?
	if self.value == nil then
		return nil
	end

	if registry == nil then
		registry = typeRegistry.default
	end
	assert(registry ~= nil, "Luau")

	local typeName = _AnyImpl.typeUrlToTypeName(self.type_url)
	local payloadType = registry:findMessage(typeName)

	if payloadType == nil then
		error(`Unknown type: \`{typeName}\``)
	end

	return payloadType.decode(self.value)
end

function _AnyImpl.isA(self: Any, typeName: string): boolean
	if self.type_url == typeName then
		return true
	end

	local suffix = "/" .. typeName
	return self.type_url:sub(-#suffix) == suffix
end

function _AnyImpl.typeUrlToTypeName(typeUrl: string): string
	local typeName = typeUrl:match("([^/]+)$")
	if typeName == nil then
		error(`Invalid type URL: \`{typeUrl}\``)
	end
	return typeName
end
