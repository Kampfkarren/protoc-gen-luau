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
	indentCount -= 1
end

function tests.it(message: string, callback: () -> ())
	local success, problem = pcall(callback)

	print(`{indent()}{if success then "✅" else "❌"} {message}`)

	if not success then
		anyFailed = true
		print(problem)
	end
end

local function deepEqual<T>(x: T, y: T): boolean
	if typeof(x) == "table" and typeof(y) == "table" then
		for key, value in x :: any do
			if not deepEqual(value, y[key]) then
				return false
			end
		end

		for key in y :: any do
			if x[key] == nil then
				return false
			end
		end

		return true
	end

	return x == y
end

function tests.assertEquals<T>(x: T, y: T)
	if not deepEqual(x, y) then
		error(`Assertion failed. Expected {x} to equal {y}`)
	end
end

function tests.finish()
	if anyFailed then
		error("Tests failed")
	end
end

return tests
