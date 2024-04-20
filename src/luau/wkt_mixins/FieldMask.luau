function _FieldMaskImpl.jsonEncode(fieldMask: FieldMask): string
	return table.concat(fieldMask.paths, ",")
end

function _FieldMaskImpl.jsonDecode(anyInput: any): FieldMask
	local input: string = anyInput

	local paths = {}
	for path in string.gmatch(input, "[^,]+") do
		table.insert(paths, path)
	end

	return FieldMask.new({
		paths = paths,
	})
end
