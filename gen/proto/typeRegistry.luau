--!strict

type _TypeRegistryImpl = {
  __index: _TypeRegistryImpl,
  new: (types: {table}) -> TypeRegistry,

  -- Finds as message based on the qualified name of the message type.
  --
  -- Example: registry.findMessage("google.protobuf.Duration")
  findMessage: (self: TypeRegistry, name: string) -> table,
}

export type TypeRegistry = typeof(setmetatable(
	{} :: {
    types: {[string]: table}
  },
	{} :: _TypeRegistryImpl
))

local TypeRegistry: _TypeRegistryImpl = {} :: _TypeRegistryImpl
TypeRegistry.__index = TypeRegistry

function TypeRegistry.new(types)
  local typeMap: {[string]: table} = {}
  for _, type in pairs(types) do
    typeMap[type.descriptor().full_name()] = type
  end

	return setmetatable({
		types = typeMap,
	}, TypeRegistry)
end

function TypeRegistry.findMessage(self, name)
  return self.types and self.types[name]
end

return {
  TypeRegistry = TypeRegistry,
}
