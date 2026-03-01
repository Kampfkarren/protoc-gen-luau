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
	local success, problemWithStackTrace = xpcall(function()
		callback()
	end, function(problem)
		return debug.traceback(problem)
	end)

	print(`{indent()}{if success then "✅" else "❌"} {message}`)

	if not success then
		anyFailed = true
		print(problemWithStackTrace)
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
	elseif typeof(x) == "buffer" and typeof(y) == "buffer" then
		return buffer.tostring(x) == buffer.tostring(y)
	end

	return x == y
end

function tests.assertEquals<T>(x: T, y: T)
	if not deepEqual(x, y) then
		error(`Assertion failed. Expected \`{x}\` to equal \`{y}\``)
	end
end

function tests.assertNotEquals<T>(x: T, y: T)
	if deepEqual(x, y) then
		error(`Assertion failed. Expected \`{x}\` not to equal \`{y}\``)
	end
end

function tests.assertStringContains(haystack: string, needle: string)
	if not string.find(haystack, needle) then
		error(`Assertion failed. Expected \`{haystack}\` to contain \`{needle}\``)
	end
end

function tests.assertThrows(callback: () -> ()): any?
	local thrown = nil
	local success = xpcall(callback, function(problem)
		thrown = problem
	end)

	if success then
		error("Expected an error to be thrown, but it wasn't")
	end

	return thrown
end

function tests.finish()
	if anyFailed then
		error("Tests failed")
	end
end

return tests
