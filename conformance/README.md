# Conformance tests

## Running the tests

1. Install
   [Lune](https://lune-org.github.io/docs/getting-started/1-installation).
1. Download or build the conformance test runner from
   [protobuf-conformance](https://github.com/bufbuild/protobuf-conformance/releases).
1. ??? (something about building the conformance protos in
   conformance/generated)
1. `cd conformance`
1. `conformance_test_runner conformance.py`

   **Note:** The test runner may be in a different directory, so your command
   may actually look something like
   `../../protobuf-conformance/.tmp/bin/conformance_test_runner conformance.py`.

## Conformance status

See
[conformance/failing_tests.txt](https://github.com/Kampfkarren/protoc-gen-luau/blob/main/conformance/failing_tests.txt)
for a list of currently failing conformance tests.
