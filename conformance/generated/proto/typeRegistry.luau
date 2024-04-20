--!strict
local TypeRegistry = {}
TypeRegistry.__index = TypeRegistry

local message = require("./message")

export type TypeRegistry = typeof(setmetatable(
	{} :: {
		types: { [string]: message.Message<unknown, unknown> },
	},
	TypeRegistry
))

function TypeRegistry.new(inputTypes: { message.Message<unknown, unknown> })
	local types = {}

	for _, inputType in inputTypes do
		types[inputType.descriptor.fullName] = inputType
	end

	return setmetatable({
		types = types,
	}, TypeRegistry)
end

function TypeRegistry.findMessage(self: TypeRegistry, fullName: string): message.Message<unknown, unknown>?
	return self.types[fullName]
end

return {
	TypeRegistry = TypeRegistry,
}
