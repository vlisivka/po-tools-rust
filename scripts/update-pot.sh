#!/bin/bash
# Script to extract translatable strings and update the POT file template.
# Requires 'xtr' (cargo install xtr)

set -ueE -o pipefail

PROJECT_ROOT="$(dirname "$0")/.."
LOCALES_DIR="$PROJECT_ROOT/locales"
POT_FILE="$LOCALES_DIR/messages.pot"

mkdir -p "$LOCALES_DIR"

echo "Extracting strings from source code..."
find "$PROJECT_ROOT/src" -name "*.rs" | xargs xtr -o "$POT_FILE"

echo "POT file updated: $POT_FILE"

# Optional: Update existing PO files if they exist
for po_file in "$LOCALES_DIR"/*.po; do
    if [ -f "$po_file" ]; then
        echo "Updating $po_file..."
        msgmerge --update --backup=none "$po_file" "$POT_FILE"
    fi
done

echo "Done."
