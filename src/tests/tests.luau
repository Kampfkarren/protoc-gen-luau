-- Jest does not yet work in Lune.
-- Replace this when it can.
local tests = {}

local anyFailed = false
local indentCount = 0

local function indent()
	return string.rep("\t", indentCount)
end

function tests.describe(category: string, callback: () -> ())
	print(`{indent()}{category}`)
	indentCount += 1
	callback()
end

function tests.it(message: string, callback: () -> ())
	local success, problem = pcall(callback)

	print(`{indent()}{if success then "✅" else "❌"} {message}`)

	if not success then
		anyFailed = true
		print(problem)
	end
end

function tests.assertEquals<T>(x: T, y: T)
	if x ~= y then
		error(`Assertion failed. Expected {x} to equal {y}`)
	end
end

function tests.assertTablesEquivalent(x: table, y: table)
	if x == y then
		return
	end

	if type(x) ~= "table" or type(y) ~= "table" then
		error("Expected both arguments to be tables")
	end

	for key, value in pairs(x) do
		if y[key] == nil then
			error(`Expected key {key} to be in both tables`)
		end

		if type(value) == "table" then
			tests.assertTablesEquivalent(value, y[key])
		else
			if value ~= y[key] then
				error(`Expected {value} to equal {y[key]}`)
			end
		end
	end

	for key, value in pairs(y) do
		if x[key] == nil then
			error(`Expected key {key} to be in both tables`)
		end
	end
end

function tests.finish()
	if anyFailed then
		print("==============")
		error("Tests failed")
	end
end

return tests
