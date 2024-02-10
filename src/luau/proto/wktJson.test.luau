--!strict
local wktJson = require("./wktJson")

local function assertEquals<T>(x: T, y: T)
	if x ~= y then
		error(`{x} ~= {y}`, 2)
	end
end

assertEquals(
	wktJson.Duration.serialize({
		seconds = 1,
	}),
	"1s"
)

assertEquals(
	wktJson.Duration.serialize({
		seconds = 1,
		nanos = 123,
	}),
	"1.000000123s"
)

local deserializedWithNanos = wktJson.Duration.deserialize("1.000000123s")
assertEquals(deserializedWithNanos.seconds, 1)
assertEquals(deserializedWithNanos.nanos, 123)
