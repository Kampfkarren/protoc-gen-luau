--!strict
local wktJson = {}

export type WellKnownType<T, TSerialized> = {
	serialize: (T) -> TSerialized,
	deserialize: <M>(TSerialized, new: (T) -> M) -> M,
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
	seconds: number,
	nanos: number,
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

	deserialize = function<M>(durationString: string, new: (Duration) -> M): M
		local simpleSecondsText = string.match(durationString, "^([%-0-9]+)s$")
		if simpleSecondsText ~= nil then
			local simpleSeconds = assert(
				tonumber(simpleSecondsText),
				"Invalid duration string received--was formatted as just having seconds, but wasn't a properly formatted int"
			)

			return new({
				seconds = simpleSeconds,
				nanos = 0,
			})
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
	seconds: number,
	nanos: number,
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
			if timestamp.nanos == 0 then "" else serializeFractionalNanos(timestamp.nanos)
		)
	end,

	deserialize = function<M>(text: string, new: (Timestamp) -> M): M
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

		return new({
			seconds = totalSeconds,
			nanos = nanos,
		})
	end,
} :: WellKnownType<Timestamp, string>

type RawValue<T> = WellKnownType<{ value: T }, T>
local function createRawValue<T>(): RawValue<T>
	return {
		serialize = function(input: { value: T }): T
			return input.value
		end,

		deserialize = function<M>(value: T, new: (T) -> M): M
			return new({
				value = value,
			})
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

type NullValue = "NULL_VALUE" | number
wktJson.NullValue = {
	serialize = function()
		return nil
	end,

	deserialize = function()
		return "NULL_VALUE"
	end,
} :: WellKnownType<NullValue, nil>

type Value = {
	kind: (
		{ type: "null_value", value: NullValue }
		| { type: "number_value", value: number }
		| { type: "string_value", value: string }
		| { type: "bool_value", value: boolean }
		| { type: "struct_value", value: Struct }
		| { type: "list_value", value: ListValue }
	)?,
}

-- Going to redeclare soon
wktJson.ListValue = {} :: WellKnownType<ListValue, { any }>
wktJson.Struct = {} :: WellKnownType<Struct, { [string]: any }>

wktJson.Value = {
	serialize = function(input: Value)
		local kind = input.kind

		if kind == nil or kind.type == "null_value" then
			return nil
		elseif kind.type == "number_value" or kind.type == "string_value" or kind.type == "bool_value" then
			return kind.value
		elseif kind.type == "list_value" then
			return wktJson.ListValue.serialize(kind.value)
		elseif kind.type == "struct_value" then
			return wktJson.Struct.serialize(kind.value)
		else
			local exhaustiveMatch: never = kind.type
			return exhaustiveMatch
		end
	end,

	deserialize = function<M>(input: any, new: (Value) -> M): M
		if input == nil then
			return new({ kind = { type = "null_value", value = "NULL_VALUE" } })
		elseif typeof(input) == "number" then
			return new({ kind = { type = "number_value", value = input } })
		elseif typeof(input) == "string" then
			return new({ kind = { type = "string_value", value = input } })
		elseif typeof(input) == "boolean" then
			return new({ kind = { type = "bool_value", value = input } })
		elseif typeof(input) == "table" then
			local isArray = true
			local count = 0
			for _ in input do
				count += 1
				if count > #input then
					isArray = false
					break
				end
			end

			isArray = count == #input

			if isArray then
				return new({ kind = { type = "list_value", value = wktJson.ListValue.deserialize(input) } })
			else
				return new({ kind = { type = "struct_value", value = wktJson.Struct.deserialize(input) } })
			end
		else
			error(`Unknown input type: {typeof(input)}`)
		end
	end,
} :: WellKnownType<Value, any>

type Struct = {
	fields: { [string]: Value },
}
wktJson.Struct = {
	serialize = function(struct: Struct): { [string]: any }
		local serialized = {}

		for key, value in struct.fields do
			serialized[key] = wktJson.Value.deserialize(value)
		end

		return serialized
	end,

	deserialize = function<M>(input: { [string]: any }, new: (Struct) -> M): M
		local fields = {}

		for key, serializedValue in input do
			fields[key] = wktJson.Value.deserialize(serializedValue)
		end

		return new({
			fields = fields,
		})
	end,
}

type ListValue = {
	values: { Value },
}
wktJson.ListValue = {
	serialize = function(list: ListValue): { any }
		local serialized = {}

		for _, value in list.values do
			table.insert(serialized, wktJson.Value.serialize(value))
		end

		return serialized
	end,

	deserialize = function<M>(input: { any }, new: (ListValue) -> M): M
		local values = {}

		for _, value in input do
			table.insert(values, wktJson.Value.deserialize(value))
		end

		return new({
			values = values,
		})
	end,
} :: WellKnownType<ListValue, { any }>

type FieldMask = {
	paths: { string },
}
wktJson.FieldMask = {
	serialize = function(fieldMask: FieldMask): string
		return table.concat(fieldMask.paths, ",")
	end,

	deserialize = function<M>(input: string, new: (FieldMask) -> M): M
		local paths = {}
		for path in string.gmatch(input, "[^,]+") do
			table.insert(paths, path)
		end

		return new({
			paths = paths,
		})
	end,
} :: WellKnownType<FieldMask, string>

return wktJson
