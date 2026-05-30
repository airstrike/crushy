#!/bin/sh
# Bump the pinned nightly to today's date, following the scheme where `master`
# always tracks the latest supported nightly and a tag preserves each old pin so
# consumers on an older nightly can `git checkout <tag> && cargo install --path .`.
#
# Steps:
#   1. Tag the current commit with the CURRENT pin (keeps the old nightly
#      installable from history).
#   2. `cargo dev sync update-nightly` rewrites the pin to today's date.
#   3. Sync the concrete dates in README.md.
#   4. Install the new toolchain and verify crushy still builds against it.
#   5. On success, commit the bump. If the build fails (the rustc_private API
#      changed), stop with the changes left in place so you can port the source.
#
# It never pushes — it prints the push/install commands to run afterwards.
set -eu

cd "$(dirname "$0")/.."

PIN_FILE="rust-toolchain.toml"

OLD=$(grep -oE 'nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}' "$PIN_FILE" | head -1)
[ -n "$OLD" ] || { echo "error: no pin found in $PIN_FILE" >&2; exit 1; }

[ -z "$(git status --porcelain)" ] || {
	echo "error: working tree is dirty; commit or stash first" >&2
	exit 1
}

echo ">> current pin: $OLD"

# 1. Preserve the old pin as a tag at the current commit.
if git rev-parse -q --verify "refs/tags/$OLD" >/dev/null; then
	echo ">> tag $OLD already exists, leaving it"
else
	git tag "$OLD"
	echo ">> tagged HEAD as $OLD"
fi

# 2. Rewrite the pin to today.
cargo dev sync update-nightly
NEW=$(grep -oE 'nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}' "$PIN_FILE" | head -1)
if [ "$NEW" = "$OLD" ]; then
	echo ">> pin already at $NEW, nothing to bump"
	exit 0
fi
echo ">> new pin: $NEW"

# 3. Keep README's copy-pasteable dates in sync (portable sed -i).
sed -i.bak "s/$OLD/$NEW/g" README.md && rm -f README.md.bak

# 4. Install the new toolchain (with the components crushy needs) and build.
rustup toolchain install "$NEW" --profile minimal \
	--component rustc-dev rust-src llvm-tools rustfmt
if ! cargo build --release; then
	cat >&2 <<EOF

error: crushy does not build against $NEW.
The rustc_private API likely changed — port the source, then re-run this script.
Your changes (pin bump + tag $OLD) are left in place.
EOF
	exit 1
fi

# 5. Commit the bump. Pushing is left to you.
git add "$PIN_FILE" crushy_utils/README.md README.md
git commit -m "chore: bump pinned nightly to $NEW"

REMOTE=$(git rev-parse --abbrev-ref '@{u}' 2>/dev/null | cut -d/ -f1)
REMOTE=${REMOTE:-origin}

cat <<EOF

Bumped $OLD -> $NEW and committed. Next:
  git push $REMOTE                 # publish the bump
  git push $REMOTE $OLD            # publish the tag preserving the old pin
  cargo install --path . --force   # reinstall the driver for $NEW
EOF
