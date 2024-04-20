set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

default:
	just --list

generate-conformance:
	rm -rf conformance/generated || true
	mkdir conformance/generated
	cargo build --release
	protoc -Iconformance/protos conformance.proto test_messages_proto3.proto --luau_out=conformance/generated --plugin=protoc-gen-luau=./target/release/protoc-gen-luau

run-conformance-tests: generate-conformance
	cd conformance && ./runner/bin/conformance_test_runner conformance.py

# Change workflows/ci.yml if you change this.
luau:
	luau-lsp analyze --settings ./.vscode/settings.json --flag:LuauTinyControlFlowAnalysis=True --flag:LuauInstantiateInSubtyping=True ./conformance/generated ./src/luau/proto ./src/tests
