--!strict

local TypeRegistry = {}
TypeRegistry.__index = TypeRegistry

local message = require("./message")

export type TypeRegistry = typeof(setmetatable(
	{} :: {
		types: { [string]: message.Message<unknown> },
	},
	TypeRegistry
))

function TypeRegistry.new(inputTypes: { message.Message<unknown> })
	local types = {}

	for _, inputType in inputTypes do
		types[inputType.descriptor.fullName] = inputType
	end

	return setmetatable({
		types = types,
	}, TypeRegistry)
end

function TypeRegistry.findMessage(self: TypeRegistry, fullName: string): message.Message<unknown>?
	return self.types[fullName]
end

function TypeRegistry.register(self: TypeRegistry, inputType: message.Message<unknown>)
	self.types[inputType.descriptor.fullName] = inputType
end

local default = TypeRegistry.new({})

return {
	TypeRegistry = TypeRegistry,
	default = default,
}
