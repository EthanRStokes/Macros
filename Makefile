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
	sudo -E install -D $(TARGET) /usr/local/bin/macros

.PHONY: all build run clean install