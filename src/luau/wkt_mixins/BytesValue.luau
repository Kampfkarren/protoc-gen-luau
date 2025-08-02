function _BytesValueImpl.jsonEncode(self: BytesValue): string
	return buffer.tostring(self.value)
end

function _BytesValueImpl.jsonDecode(anyValue: any): BytesValue
	local value: string = anyValue
	return _BytesValueImpl.new({
		value = buffer.fromstring(value),
	})
end
