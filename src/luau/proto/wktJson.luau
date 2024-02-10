--!strict
local wktJson = {}

export type WellKnownType<T, TSerialized> = {
	serialize: (T) -> TSerialized,
	deserialize: (TSerialized) -> T,
}

type Duration = {
	seconds: number?,
	nanos: number?,
}
wktJson.Duration = {
	serialize = function(duration: Duration): string
		assert(
			duration.seconds == nil or (duration.seconds >= -315576000000 and duration.seconds <= 315576000000),
			"Seconds out of range"
		)
		assert(
			duration.nanos == nil or (duration.nanos >= -999999999 and duration.nanos <= 999999999),
			"Nanoseconds out of range"
		)

		-- TODO: For durations of one second or more, a non-zero value for the nanos field must be of the same sign as the seconds field.
		if duration.nanos == nil or duration.nanos == 0 or duration.nanos % 1e9 == 0 then
			return `{duration.seconds}s`
		end

		local nanoSign = if duration.seconds == 0 and math.sign(duration.nanos) == -1 then "-" else ""

		if duration.nanos % 1e6 == 0 then
			return string.format("%s%d.%03ds", nanoSign, duration.seconds or 0, duration.nanos)
		end

		if duration.nanos % 1e3 == 0 then
			return string.format("%s%d.%06ds", nanoSign, duration.seconds or 0, duration.nanos)
		end

		return string.format("%s%d.%09ds", nanoSign, duration.seconds or 0, duration.nanos)
	end,

	deserialize = function(durationString: string): Duration
		local simpleSecondsText = string.match(durationString, "^([%-0-9]+)s$")
		if simpleSecondsText ~= nil then
			local simpleSeconds = assert(
				tonumber(simpleSecondsText),
				"Invalid duration string received--was formatted as just having seconds, but wasn't a properly formatted int"
			)

			return {
				seconds = simpleSeconds,
				nanos = nil,
			}
		end

		local secondsText, nanosText = string.match(durationString, "^([%-0-9]+)%.([0-9]+)s$")
		assert(nanosText ~= nil, "Invalid duration string received--nanos provided are invalid")

		local seconds = assert(tonumber(secondsText), "Invalid duration string received--seconds provided are invalid")

		local nanos =
			assert(tonumber(nanosText), "Invalid duration string received--nanos provided couldn't turn into a number")
		for digit = #nanosText + 1, 9 do
			nanos += 10 ^ (digit - 1)
		end

		return {
			seconds = seconds,
			nanos = nanos,
		}
	end,
} :: WellKnownType<Duration, string>

return wktJson
