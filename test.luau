-- --!strict

local any = require('gen/google/protobuf/any')
local duration = require('gen/google/protobuf/duration')
local wrappers = require('gen/google/protobuf/wrappers')
local descriptor = require('gen/proto/descriptor')
local typeRegistry = require('gen/proto/typeRegistry')

print(descriptor)

local forty_two = wrappers.Int32Value.new({value = 42})
print(forty_two)
print(forty_two:jsonEncode())
print(wrappers.Int32Value.jsonDecode(forty_two:jsonEncode()))
print(wrappers.Int32Value.jsonDecode(forty_two:jsonEncode()):jsonEncode())


print(duration)
local one_hour = duration.Duration.new()
one_hour.seconds = 3600
print(one_hour)

print(one_hour:jsonEncode())
print(duration.Duration.jsonDecode(one_hour:jsonEncode()))
print(duration.Duration.jsonDecode(one_hour:jsonEncode()):jsonEncode())

print(one_hour.descriptor().name())
print(one_hour.descriptor().full_name())


print(duration.Duration.encode(one_hour))
print(one_hour:encode())
print(typeof(one_hour:encode()))
one_hour_encoded = one_hour:encode()
print(typeof(one_hour_encoded))
print('asdf')
print(duration.Duration.decode(one_hour:encode()))
one_hour_decoded = duration.Duration.new()
print(one_hour_decoded.decode(one_hour:encode()))
print(one_hour_decoded)

registry = typeRegistry.TypeRegistry.new({
    duration.Duration,
    any.Any,
})
print('** type registry')
print(registry:findMessage("google.protobuf.Duration").decode(duration.Duration.new({seconds = 3600}):encode()))

print('** any')
local packed_duration = any.Any.new()
print(packed_duration)
print(any.Any.encode(packed_duration))
packed_duration:pack(one_hour)
print(packed_duration:pack(one_hour))
print(packed_duration)

print(packed_duration:unpack(registry))

local json_packed = packed_duration:jsonEncode({registry = registry})

print(json_packed)

print(any.Any.jsonDecode(json_packed, {registry = registry}))

