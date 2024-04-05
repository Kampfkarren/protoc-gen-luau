--!strict

type _DescriptorImpl = {
  __index: _DescriptorImpl,
  new: () -> Descriptor,

  -- The unqualified name of the message type.
  name: () -> string,

  -- The qualified name of the message type, scope delimited by periods.
  full_name: () -> string,
}

export type Descriptor = typeof(setmetatable(
	{} :: {
	},
	{} :: _DescriptorImpl
))

local Descriptor: _DescriptorImpl = {} :: _DescriptorImpl
Descriptor.__index = Descriptor

function Descriptor.new()
	return setmetatable({
		name = function() return "" end,
		full_name = function() return "" end,
	}, Descriptor)
end

return {
  Descriptor = Descriptor,
}