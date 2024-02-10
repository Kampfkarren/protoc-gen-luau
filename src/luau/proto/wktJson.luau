--!strict
local wktJson = {}

export type WellKnownType<T, TSerialized> = {
	serialize: (T) -> TSerialized,
	deserialize: (TSerialized) -> T,
}

local function serializeFractionalNanos(nanos: number): string
	if nanos % 1e6 == 0 then
		return string.format(".%03ds", nanos)
	elseif nanos % 1e3 == 0 then
		return string.format(".%06ds", nanos)
	else
		return string.format(".%09ds", nanos)
	end
end

local function deserializeFractionalNanos(nanosText: string): number
	local nanos = assert(tonumber(nanosText), "Invalid nanos string received, couldn't turn into a number")
	for digit = #nanosText + 1, 9 do
		nanos += 10 ^ (digit - 1)
	end

	return nanos
end

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
		return string.format("%s%d%s", nanoSign, duration.seconds or 0, serializeFractionalNanos(duration.nanos))
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

		return {
			seconds = seconds,
			nanos = deserializeFractionalNanos(nanosText),
		}
	end,
} :: WellKnownType<Duration, string>

type Timestamp = {
	seconds: number?,
	nanos: number?,
}
wktJson.Timestamp = {
	serialize = function(timestamp: Timestamp): string
		local dateInfo = os.date("!*t", timestamp.seconds or 0)
		return string.format(
			"%04d-%02d-%02dT%02d:%02d:%02d%sZ",
			dateInfo.year,
			dateInfo.month,
			dateInfo.day,
			dateInfo.hour,
			dateInfo.min,
			dateInfo.sec,
			if timestamp.nanos == nil or timestamp.nanos == 0 then "" else serializeFractionalNanos(timestamp.nanos)
		)
	end,

	deserialize = function(text: string): Timestamp
		local year, month, day, hour, min, sec, nanosWithDot =
			string.match(text, "^([0-9]+)-([0-9]+)-([0-9]+)T([0-9]+):([0-9]+):([0-9]+)(.*)Z$")

		if
			year == nil
			or month == nil
			or day == nil
			or hour == nil
			or min == nil
			or sec == nil
			or nanosWithDot == nil
		then
			error("Invalid timestamp provided")
		end

		local nanos = 0
		if #nanosWithDot ~= 0 then
			local nanosText = string.match(text, "^%.([0-9]+)$")
			if nanosText == nil then
				error("Invalid nanoseconds provided in timestamp")
			end

			nanos = deserializeFractionalNanos(nanosText)
		end

		local totalSeconds = os.time({
			year = assert(tonumber(year), "Invalid formatted year"),
			month = assert(tonumber(month), "Invalid formatted month"),
			day = assert(tonumber(day), "Invalid formatted day"),
			hour = assert(tonumber(hour), "Invalid formatted hour"),
			min = assert(tonumber(min), "Invalid formatted min"),
			sec = assert(tonumber(sec), "Invalid formatted sec"),
		})

		return {
			sec = totalSeconds,
			nanos = nanos,
		}
	end,
} :: WellKnownType<Timestamp, string>

type RawValue<T> = WellKnownType<{ value: T }, T>
local function createRawValue<T>(): RawValue<T>
	return {
		serialize = function(input: { value: T }): T
			return input.value
		end,

		deserialize = function(value: T): { value: T }
			return {
				value = value,
			}
		end,
	}
end

wktJson.BoolValue = createRawValue() :: RawValue<boolean>
wktJson.BytesValue = createRawValue() :: RawValue<buffer>
wktJson.DoubleValue = createRawValue() :: RawValue<number>
wktJson.FloatValue = createRawValue() :: RawValue<number>
wktJson.Int32Value = createRawValue() :: RawValue<number>
wktJson.Int64Value = createRawValue() :: RawValue<number>
wktJson.UInt32Value = createRawValue() :: RawValue<number>
wktJson.UInt64Value = createRawValue() :: RawValue<number>
wktJson.StringValue = createRawValue() :: RawValue<string>

return wktJson
