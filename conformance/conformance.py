#!/usr/bin/env python3
import os
import subprocess
import sys
import tempfile

with tempfile.NamedTemporaryFile() as f:
	while True:
		length_bytes = sys.stdin.buffer.read(4)
		length = int.from_bytes(length_bytes, byteorder='little')

		if length == 0:
			break

		message = sys.stdin.buffer.read(length)

		if os.environ.get("SAVE_LAST_MESSAGE"):
			with open("./last_message.bin", "wb") as last_message_file:
				last_message_file.write(message)

		f.write(message)
		f.flush()
		subprocess.run(["lune", "run", "conformance_test.luau", f.name])
