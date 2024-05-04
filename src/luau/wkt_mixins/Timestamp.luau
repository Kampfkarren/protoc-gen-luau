local function serializeFractionalNanos(nanos: number): string
	if nanos % 1e6 == 0 then
		return string.format(".%03d", nanos)
	elseif nanos % 1e3 == 0 then
		return string.format(".%06d", nanos)
	else
		return string.format(".%09d", nanos)
	end
end

local function deserializeFractionalNanos(nanosText: string): number
	local nanos = assert(tonumber(nanosText), "Invalid nanos string received, couldn't turn into a number")
	for digit = #nanosText + 1, 9 do
		nanos += 10 ^ (digit - 1)
	end

	return nanos
end

function _TimestampImpl.jsonEncode(timestamp: Timestamp): string
	if timestamp.seconds > 253402300799 then
		error("Invalid timestamp provided: years after 9999 are not supported")
	end
	if timestamp.seconds < -62135596800 then
		error("Invalid timestamp provided: years before 0001 are not supported")
	end
	local dateInfo = os.date("!*t", timestamp.seconds or 0)
	if dateInfo == nil then
		error("Invalid timestamp provided: exceeded supported timestamp range on this platform")
	end
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
end

function _TimestampImpl.jsonDecode(anyValue: any): Timestamp
	local text: string = anyValue

	local year, month, day, hour, min, sec, nanosWithDot =
		string.match(text, "^([0-9]+)-([0-9]+)-([0-9]+)T([0-9]+):([0-9]+):([0-9]+)(.*)Z$")

	if year == nil or month == nil or day == nil or hour == nil or min == nil or sec == nil or nanosWithDot == nil then
		error("Invalid timestamp provided")
	end

	local nanos = 0
	if #nanosWithDot ~= 0 then
		local nanosText = string.match(nanosWithDot, "^%.([0-9]+)$")
		if nanosText == nil then
			error("Invalid nanoseconds provided in timestamp: `" .. nanosWithDot .. "`")
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

	return Timestamp.new({
		seconds = totalSeconds,
		nanos = nanos,
	})
end
