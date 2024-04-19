-- Converts a number of nanos to a string representation.
-- Sign and trailing zeroes are dropped.
-- 500000000 -> ".5s"
-- 500000001 -> ".500000001s"
-- 0 -> ".0s"
-- 1 -> ".000000001s"
-- -1 -> "-.000000001s"
local function serializeFractionalNanos(nanos: number): string
  nanos = nanos * math.sign(nanos)

  local nanosText = tostring(nanos)
  local nanosTextLength = #nanosText

  local leadingZeroes = 9 - nanosTextLength
  local leadingZeroesText = string.rep("0", leadingZeroes)

  -- remove trailing zeroes
  while nanosText:sub(-1) == "0" do
    nanosText = nanosText:sub(1, -2)
  end

  return "." .. leadingZeroesText .. nanosText .. "s"
end

local function deserializeFractionalNanos(nanosText: string): number
	local nanos = assert(tonumber(nanosText), "Invalid nanos string received, couldn't turn into a number")
	for digit = #nanosText + 1, 9 do
		nanos = nanos * 10
	end

	return nanos
end

function Duration.jsonEncode(duration: Duration): string
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
end

function Duration.jsonDecode(durationString: string): Duration
  local simpleSecondsText = string.match(durationString, "^([%-0-9]+)s$")
  if simpleSecondsText ~= nil then
    local simpleSeconds = assert(
      tonumber(simpleSecondsText),
      "Invalid duration string received--was formatted as just having seconds, but wasn't a properly formatted int"
    )

    return Duration.new({
      seconds = simpleSeconds,
      nanos = 0,
    })
  end

  local secondsText, nanosText = string.match(durationString, "^([%-0-9]+)%.([0-9]+)s$")
  assert(nanosText ~= nil, "Invalid duration string received--nanos provided are invalid")

  local seconds = assert(tonumber(secondsText), "Invalid duration string received--seconds provided are invalid")
  local nanos = deserializeFractionalNanos(nanosText) * (secondsText:sub(1, 1) == "-" and -1 or 1)

  return Duration.new({
    seconds = seconds,
    nanos = nanos,
  })
end
