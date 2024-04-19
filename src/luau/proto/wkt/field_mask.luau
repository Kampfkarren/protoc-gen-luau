function FieldMask.jsonEncode(fieldMask: FieldMask): string
  return table.concat(fieldMask.paths, ",")
end

function FieldMask.jsonDecode(input: string): FieldMask
  local paths = {}
  for path in string.gmatch(input, "[^,]+") do
    table.insert(paths, path)
  end

  return FieldMask.new({
    paths = paths,
  })
end
