# Makefile for the Macros project

# Variables
CARGO := cargo
TARGET := target/release/macros

# Default target
all: build

# Build the project
build:
	$(CARGO) build --release

# Run the project
run: build
	$(TARGET)

# Clean the project
clean:
	$(CARGO) clean

# Install the project
install: build
	sudo -E install -Dm0755 $(TARGET) /usr/local/bin/macros
	sudo -E install -Dm0644 res/macros.desktop /usr/share/applications/macros.desktop

uninstall:
	sudo -E rm -f /usr/local/bin/macros
	sudo -E rm -f /usr/share/applications/macros.desktop

.PHONY: all build run clean install