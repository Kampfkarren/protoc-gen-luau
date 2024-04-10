default:
	just --list

generate-conformance:
	rm -rf conformance/generated || true
	mkdir conformance/generated
	cargo build --release
	protoc -Iconformance/protos conformance.proto test_messages_proto3.proto --luau_out=conformance/generated --plugin=protoc-gen-luau=./target/release/protoc-gen-luau
	cd conformance &&	./runner/bin/conformance_test_runner conformance.py
