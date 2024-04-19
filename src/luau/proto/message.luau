export type Descriptor = {
	name: string,
	fullName: string,
}

export type Message<T, Fields> = {
	new: (fields: Fields?) -> T,

	encode: (T) -> buffer,
	decode: (buffer) -> T,

	jsonEncode: (T) -> any,
	jsonDecode: ({ [string]: any }) -> T,

	descriptor: Descriptor,
}

return nil
