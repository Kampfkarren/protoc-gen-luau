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
	failureSet.failure = {
		-- Any is not supported
		"Required.Proto3.JsonInput.Any.JsonOutput",
		"Required.Proto3.JsonInput.Any.ProtobufOutput",
		"Required.Proto3.JsonInput.AnyNested.JsonOutput",
		"Required.Proto3.JsonInput.AnyNested.ProtobufOutput",
		"Required.Proto3.JsonInput.AnyUnorderedTypeTag.JsonOutput",
		"Required.Proto3.JsonInput.AnyUnorderedTypeTag.ProtobufOutput",
		"Required.Proto3.JsonInput.AnyWithDuration.JsonOutput",
		"Required.Proto3.JsonInput.AnyWithDuration.ProtobufOutput",
		"Required.Proto3.JsonInput.AnyWithFieldMask.JsonOutput",
		"Required.Proto3.JsonInput.AnyWithFieldMask.ProtobufOutput",
		"Required.Proto3.JsonInput.AnyWithInt32ValueWrapper.JsonOutput",
		"Required.Proto3.JsonInput.AnyWithInt32ValueWrapper.ProtobufOutput",
		"Required.Proto3.JsonInput.AnyWithStruct.JsonOutput",
		"Required.Proto3.JsonInput.AnyWithStruct.ProtobufOutput",
		"Required.Proto3.JsonInput.AnyWithTimestamp.JsonOutput",
		"Required.Proto3.JsonInput.AnyWithTimestamp.ProtobufOutput",
		"Required.Proto3.JsonInput.AnyWithValueForInteger.JsonOutput",
		"Required.Proto3.JsonInput.AnyWithValueForInteger.ProtobufOutput",
		"Required.Proto3.JsonInput.AnyWithValueForJsonObject.JsonOutput",
		"Required.Proto3.JsonInput.AnyWithValueForJsonObject.ProtobufOutput",
	}

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

local ok, problem: any = pcall(function()
	local testAllTypes: test_messages_proto3.TestAllTypesProto3

	assert(request.payload ~= nil, "No payload provided")
	if request.payload.type == "json_payload" then
		local success, result = pcall(net.jsonDecode, request.payload.value)
		if not success then
			response.result = { type = "parse_error", value = result }
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
			response.result = { type = "parse_error", value = result }
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
		response.result = {
			type = "json_payload",
			value = jsonEncode(test_messages_proto3.TestAllTypesProto3.jsonEncode(testAllTypes)),
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
end)

if not ok then
	response.result = {
		type = "runtime_error",
		value = tostring(problem),
	}

	writeResponse()
end
