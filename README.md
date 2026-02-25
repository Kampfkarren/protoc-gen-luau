`protoc-gen-luau` generates strictly typed Luau files from [Protobuf](https://protobuf.dev/) files.

To use, clone the repo and run `cargo install`. When you run `protoc`, add `--luau_out=path/to/protos`. For example, to export protos in `protos/` to `src/LuauProtos/`...

```
protoc -Iprotos --luau_out=src/LuauProtos
```

## Options
- Add `--luau_opt=roblox_imports=true` to indicate you are in a Roblox environment. This currently replaces `require`s from string requires to instance based requires. I'm not actually sure this is necessary anymore though.

- `--luau_opt=field_name_case=snake|camel` — Control Luau field names casing. 
  - If no option passed, the default behavior is to keep the proto name unchanged
	- `string_value` -> `string_value`
	- `FIELD_NAME` -> `FIELD_NAME`
  - `snake`: converts field names to snake case
    - `string_value` -> `string_value`
	- `FIELD_NAME` -> `field_name`
  - `camel`: converts field names to camel case
    - `string_value` -> `stringValue`
	- `FIELD_NAME` -> `fieldName`
  - Invalid values cause compilation to fail

## API

### Messages
Suppose we have the following message:
```protobuf
message Pair {
	double x = 1;
	double y = 2;
}
```
The exported script will have the following:
- Exported type `Pair` representing the Pair class
- A `Pair` class with:
	- `Pair.new(partialFields): Pair`
		- `partialFields` in this case would be `{ x: number?, y: number? }`. Anything not specified will be defaulted as per Protobuf's rules.
	- `Pair:encode(): buffer`
		- Returns a buffer representing the serialized Protobuf.
	- `Pair.decode(input: buffer): Pair`
		- Deserializes a serialized Protobuf.
	- `Pair:jsonEncode(): { [string]: any }`
		- Returns a JSON encoded representation of the message as per Protobuf's rules.
	- `Pair.jsonDecode(input: { [string]: any }): Pair`
		- Deserializes a JSON encoded representation of the message as per Protobuf's rules.
	- `Pair.descriptor: proto.Descriptor`
		- A runtime representation of what the type is--just a struct with `{ name: string, fullName: string }`.

### Enums
If we have the following:
```protobuf
enum Kind {
	A = 0,
	B = 1,
	C = 2,
}
```
The exported script will export a type `Kind` that is a union string of all the options, as well as `number` for when it is unspecified. In this case: `"A" | "B" | "C" | number`

### Any
`Any` is supported, though these docs are not ready yet.
