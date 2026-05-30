#!/bin/sh
# Bump the pinned nightly to today's date, following the scheme where `master`
# always tracks the latest supported nightly and a tag preserves each old pin so
# consumers on an older nightly can `git checkout <tag> && cargo install --path .`.
#
# Steps:
#   1. Tag the current commit with the CURRENT pin (keeps the old nightly
#      installable from history).
#   2. `cargo dev sync update_nightly` rewrites the pin to today's date.
#   3. Sync the concrete dates in README.md.
#   4. Install the new toolchain and verify crushy still builds against it.
#   5. On success, commit the bump.
#
# Local runs commit and print the push/install commands. In CI (`--ci`) nothing
# is committed; the result is reported via $GITHUB_OUTPUT for the workflow to act
# on (open a PR on success, file an issue when the build breaks):
#   status = uptodate | unavailable | broken | bumped
#   old, new = the pin dates
set -eu

cd "$(dirname "$0")/.."

CI=0
[ "${1:-}" = "--ci" ] && CI=1

PIN_FILE="rust-toolchain.toml"

emit() { [ -n "${GITHUB_OUTPUT:-}" ] && echo "$1=$2" >>"$GITHUB_OUTPUT" || true; }

OLD=$(grep -oE 'nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}' "$PIN_FILE" | head -1)
[ -n "$OLD" ] || { echo "error: no pin found in $PIN_FILE" >&2; exit 1; }
emit old "$OLD"

if [ "$CI" -eq 0 ] && [ -n "$(git status --porcelain)" ]; then
	echo "error: working tree is dirty; commit or stash first" >&2
	exit 1
fi

echo ">> current pin: $OLD"

# 1. Preserve the old pin as a tag at the current commit (pushed only on success).
if git rev-parse -q --verify "refs/tags/$OLD" >/dev/null; then
	echo ">> tag $OLD already exists, leaving it"
else
	git tag "$OLD"
	echo ">> tagged HEAD as $OLD"
fi

# 2. Rewrite the pin to today.
cargo dev sync update_nightly
NEW=$(grep -oE 'nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}' "$PIN_FILE" | head -1)
emit new "$NEW"
if [ "$NEW" = "$OLD" ]; then
	echo ">> pin already at $NEW, nothing to bump"
	emit status uptodate
	exit 0
fi
echo ">> new pin: $NEW"

# 3. Keep README's copy-pasteable dates in sync (portable sed -i).
sed -i.bak "s/$OLD/$NEW/g" README.md && rm -f README.md.bak

# 4. Install the new toolchain (with the components crushy needs).
if ! rustup toolchain install "$NEW" --profile minimal \
	--component rustc-dev rust-src llvm-tools rustfmt; then
	echo "error: could not install $NEW (not published yet?)" >&2
	emit status unavailable
	[ "$CI" -eq 1 ] && exit 0 || exit 1
fi

# 5. Verify crushy still builds against it.
if ! cargo build --release; then
	echo "error: crushy does not build against $NEW; the rustc_private API likely changed." >&2
	emit status broken
	if [ "$CI" -eq 1 ]; then
		exit 0
	fi
	echo "Your changes (pin bump + tag $OLD) are left in place; port the source and re-run." >&2
	exit 1
fi

emit status bumped

if [ "$CI" -eq 1 ]; then
	echo ">> CI mode: leaving changes uncommitted for the PR step"
	exit 0
fi

# Local: commit the bump. Pushing is left to you.
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
