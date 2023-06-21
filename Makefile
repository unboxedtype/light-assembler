build:
	cargo build --release -j 4

install:
	sudo cp ./target/release/libever_assembler.so /usr/lib64/

uninstall:
	sudo rm -f /usr/lib64/libever_assembler.so