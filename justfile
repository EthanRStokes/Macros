name := "macros"
appid := "com.treetrain1.Macros"

# Variables
TARGET := "target/release/macros"

# Default target
default: build

# Build the project
build *args:
    cargo build --release {{args}}

# Run the project
run: build
    {{TARGET}}

# Clean the project
clean:
    cargo clean

# Install the project
install:
    install -Dm0755 {{TARGET}} /usr/bin/macros
    install -Dm0644 res/macros.desktop /usr/share/applications/macros.desktop

# Uninstall the project
uninstall:
    rm /usr/bin/macros /usr/share/applications/macros.desktop