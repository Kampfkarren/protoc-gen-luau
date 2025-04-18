#!/usr/bin/env -S lune run
--!strict
local fs = require("@lune/fs")
local net = require("@lune/net")
local process = require("@lune/process")
local stdio = require("@lune/stdio")

local conformance = require("./generated/conformance")
local test_messages_proto3 = require("./generated/test_messages_proto3")

local jsonEncode = require("jsonEncode")

local file = buffer.fromstring(fs.readFile(process.args[1]))

local request = conformance.ConformanceRequest.decode(file)
local response = conformance.ConformanceResponse.new()

local function writeResponse()
	local encoded = conformance.ConformanceResponse.encode(response)
	stdio.write(string.pack("<I4", buffer.len(encoded)) .. buffer.tostring(encoded))
end

if
	request.test_category == "TEXT_FORMAT_TEST"
	or request.test_category == "JSON_IGNORE_UNKNOWN_PARSING_TEST"
	or request.test_category == "JSPB_TEST"
then
	response.result = { type = "skipped", value = "Ignorable test" }
	writeResponse()
	return
end

if request.message_type == "conformance.FailureSet" then
	local failureSet = conformance.FailureSet.new()

	response.result = {
		type = "protobuf_payload",
		value = conformance.FailureSet.encode(failureSet),
	}
	writeResponse()
	return
elseif request.message_type ~= "protobuf_test_messages.proto3.TestAllTypesProto3" then
	response.result = { type = "skipped", value = "Only proto3 is supported" }
	writeResponse()
	return
end

local ok, problem: any = xpcall(function()
	local testAllTypes: test_messages_proto3.TestAllTypesProto3

	assert(request.payload ~= nil, "No payload provided")
	if request.payload.type == "json_payload" then
		local success, result = pcall(net.jsonDecode, request.payload.value)
		if not success then
			response.result = { type = "parse_error", value = tostring(result) }
			writeResponse()
			return
		end

		if not pcall(function()
			testAllTypes = test_messages_proto3.TestAllTypesProto3.jsonDecode(result)
		end) then
			response.result = { type = "parse_error", value = "Failed to parse JSON" }
			writeResponse()
			return
		end
	elseif request.payload.type == "protobuf_payload" then
		local success, result: any = pcall(test_messages_proto3.TestAllTypesProto3.decode, request.payload.value)
		if not success then
			response.result = { type = "parse_error", value = tostring(result) }
			writeResponse()
			return
		end

		testAllTypes = result
	else
		response.result = { type = "runtime_error", value = `Unknown payload type: {request.payload.type}` }
		writeResponse()
		return
	end

	if request.requested_output_format == "JSON" then
		local json
		if
			not pcall(function()
				json = jsonEncode(test_messages_proto3.TestAllTypesProto3.jsonEncode(testAllTypes))
			end)
		then
			response.result = { type = "serialize_error", value = "Failed to encode JSON" }
			writeResponse()
			return
		end

		response.result = {
			type = "json_payload",
			value = json,
		}
	elseif request.requested_output_format == "PROTOBUF" then
		response.result = {
			type = "protobuf_payload",
			value = test_messages_proto3.TestAllTypesProto3.encode(testAllTypes),
		}
	else
		response.result =
			{ type = "runtime_error", value = `Unknown requested output type: {request.requested_output_format}` }
	end

	writeResponse()
end, function(message)
	return tostring(message) .. "\n" .. debug.traceback()
end)

if not ok then
	response.result = {
		type = "runtime_error",
		value = tostring(problem),
	}

	local clock = os.clock()
	fs.writeFile(`runtime_errors/{clock}.bin`, buffer.tostring(file))
	fs.writeFile(`runtime_errors/{clock}.txt`, `{problem}\nPayload type: {request.payload and request.payload.type}`)

	writeResponse()
end
