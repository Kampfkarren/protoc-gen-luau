function BoolValue.jsonEncode(value: BoolValue): boolean
	return value.value
end

function BoolValue.jsonDecode(value: boolean): BoolValue
	return BoolValue.new({
		value = value,
	})
end

function BytesValue.jsonEncode(value: BytesValue): buffer
	return value.value
end

function BytesValue.jsonDecode(value: buffer): BytesValue
	return BytesValue.new({
		value = value,
	})
end

function DoubleValue.jsonEncode(value: DoubleValue): number
	return value.value
end

function DoubleValue.jsonDecode(value: number): DoubleValue
	return DoubleValue.new({
		value = value,
	})
end

function FloatValue.jsonEncode(value: FloatValue): number
	return value.value
end

function FloatValue.jsonDecode(value: number): FloatValue
	return FloatValue.new({
		value = value,
	})
end

function Int32Value.jsonEncode(value: Int32Value): number
	return value.value
end

function Int32Value.jsonDecode(value: number): Int32Value
	return Int32Value.new({
		value = value,
	})
end

function Int64Value.jsonEncode(value: Int64Value): number
	return value.value
end

function Int64Value.jsonDecode(value: number): Int64Value
	return Int64Value.new({
		value = value,
	})
end

function UInt32Value.jsonEncode(value: UInt32Value): number
	return value.value
end

function UInt32Value.jsonDecode(value: number): UInt32Value
	return UInt32Value.new({
		value = value,
	})
end

function UInt64Value.jsonEncode(value: UInt64Value): number
	return value.value
end

function UInt64Value.jsonDecode(value: number): UInt64Value
	return UInt64Value.new({
		value = value,
	})
end

function StringValue.jsonEncode(value: StringValue): string
	return value.value
end

function StringValue.jsonDecode(value: string): StringValue
	return StringValue.new({
		value = value,
	})
end