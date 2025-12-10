#!/bin/bash

# Create a temporary file for build output
TEMP_OUTPUT_FILE=$(mktemp)

echo "--- Starting RocksDB Build ---"

# Run cargo build within the Nix development shell and redirect output to the temporary file
nix develop --command bash -c "CARGO_CFG_TARGET_VERBOSE=1 cargo build" > "$TEMP_OUTPUT_FILE" 2>&1

BUILD_STATUS=$?

echo "--- Build Output Summary ---"

# Filter and print errors
echo "Errors:"
grep -iE "error:|fatal error:" "$TEMP_OUTPUT_FILE" || echo "No explicit errors found."

# Filter and print warnings about duplicate directories
echo -e "\nDuplicate Directory Warnings:"
grep "duplicate directory" "$TEMP_OUTPUT_FILE" || echo "No duplicate directory warnings found."

# Print final build status
if [ $BUILD_STATUS -eq 0 ]; then
    echo -e "\n--- Build Succeeded ---"
else
    echo -e "\n--- Build Failed (Exit Code: $BUILD_STATUS) ---"
fi

# Clean up the temporary file
rm "$TEMP_OUTPUT_FILE"
