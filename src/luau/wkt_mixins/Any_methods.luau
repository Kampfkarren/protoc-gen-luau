function _AnyImpl.pack(payload: proto.Message<any, any>, typeUrlPrefix: string): Any
	return _AnyImpl.new({
		type_url = typeUrlPrefix .. "/" .. payload.descriptor.fullName,
		value = payload:encode(),
	})
end

-- Luau: It refuses to believe these are the same unpack, for some reason
local anyUnpack: typeof(({} :: _AnyImpl).unpack) = function(
	self: Any,
	registry: typeRegistry.TypeRegistry?
): proto.Message<any, any>?
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

	return payloadType.decode(self.value) :: proto.Message<any, any>
end

_AnyImpl.unpack = anyUnpack

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
