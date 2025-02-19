TARGET := target/release/macros

# Default target
all: build

# Build the project
build:
	cargo build --release

# Run the project
run: build
	$(TARGET)

# Clean the project
clean:
	cargo clean

# Install the project
install:
	install -Dm0755 $(TARGET) /usr/local/bin/macros
	install -Dm0644 res/macros.desktop /usr/share/applications/macros.desktop

uninstall:
	rm -f /usr/local/bin/macros
	rm -f /usr/share/applications/macros.desktop

.PHONY: all build run clean install