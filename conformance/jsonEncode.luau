--!strict
local serde = require("@lune/serde")

local function isArray(input: { [any]: any })
	local count = 0
	for _ in input do
		count += 1
	end

	return count == #input
end

-- Re-implementation of jsonEncode that guarantees JSON etc are round tripped
local function jsonEncode(input: any): string
	if typeof(input) == "table" then
		if next(input) ~= nil and isArray(input) then
			local items = {}
			for _, value in input do
				table.insert(items, jsonEncode(value))
			end

			return `[{table.concat(items, ",")}]`
		else
			local items = {}
			for key, value in input do
				assert(typeof(key) == "string", "JSON key needs to be a string")
				table.insert(items, `\"{key}\":{jsonEncode(value)}`)
			end

			return `\{{table.concat(items, ",")}\}`
		end
	elseif typeof(input) == "number" or typeof(input) == "boolean" then
		return tostring(input)
	elseif typeof(input) == "string" then
		return serde.encode("json", input)
	else
		error(`Unknown input type: {tostring(input)}`)
	end
end

return jsonEncode
