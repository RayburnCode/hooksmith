#!/usr/bin/env bash
# publish.sh — safe ordered publish for the hooksmith workspace
#
# Usage:
#   ./scripts/publish.sh [patch|minor|major|<x.y.z>]
#
# Defaults to `patch` if no argument is given.
#
# What it does:
#   1. Ensures the working tree is clean (no uncommitted changes)
#   2. Bumps the workspace version in Cargo.toml
#   3. Updates CHANGELOG.md (promotes [Unreleased] → [x.y.z] - YYYY-MM-DD)
#   4. Runs `cargo test --workspace` — aborts on failure
#   5. Commits the version bump
#   6. Publishes hooksmith-core first, then polls crates.io until it's indexed
#   7. Publishes discord_hook
#   8. Creates a git tag vX.Y.Z and pushes commits + tag

set -euo pipefail

# ── Colour helpers ────────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; CYAN='\033[0;36m'; NC='\033[0m'
info()    { echo -e "${CYAN}[publish]${NC} $*"; }
success() { echo -e "${GREEN}[publish]${NC} $*"; }
warn()    { echo -e "${YELLOW}[publish]${NC} $*"; }
die()     { echo -e "${RED}[publish] ERROR:${NC} $*" >&2; exit 1; }

# ── Locate workspace root (where this script lives) ───────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

# ── Parse bump argument ───────────────────────────────────────────────────────
BUMP="${1:-patch}"

bump_version() {
    local current="$1" bump="$2"
    local major minor patch
    IFS='.' read -r major minor patch <<< "$current"
    case "$bump" in
        major) echo "$((major + 1)).0.0" ;;
        minor) echo "${major}.$((minor + 1)).0" ;;
        patch) echo "${major}.${minor}.$((patch + 1))" ;;
        [0-9]*.[0-9]*.[0-9]*) echo "$bump" ;;  # explicit version passed
        *) die "Unknown bump type '$bump'. Use: patch | minor | major | x.y.z" ;;
    esac
}

# ── Read current version ──────────────────────────────────────────────────────
CARGO_TOML="$ROOT/Cargo.toml"
CURRENT_VERSION=$(grep '^version' "$CARGO_TOML" | head -1 | sed 's/.*"\(.*\)".*/\1/')
NEW_VERSION=$(bump_version "$CURRENT_VERSION" "$BUMP")

info "Bumping workspace version: ${CURRENT_VERSION} → ${NEW_VERSION}"

# ── Guard: clean working tree ─────────────────────────────────────────────────
if ! git -C "$ROOT" diff --quiet || ! git -C "$ROOT" diff --cached --quiet; then
    die "Working tree has uncommitted changes. Commit or stash them first."
fi

# ── Confirm ───────────────────────────────────────────────────────────────────
echo ""
warn "This will:"
warn "  • Bump workspace version to ${NEW_VERSION}"
warn "  • Update CHANGELOG.md"
warn "  • Run cargo test --workspace"
warn "  • Publish hooksmith-core v${NEW_VERSION}"
warn "  • Publish discord_hook v${NEW_VERSION}"
warn "  • Push a git tag v${NEW_VERSION}"
echo ""
read -rp "$(echo -e "${YELLOW}Continue? [y/N]${NC} ")" CONFIRM
[[ "${CONFIRM,,}" == "y" ]] || { info "Aborted."; exit 0; }

# ── Bump version in Cargo.toml ────────────────────────────────────────────────
sed -i.bak "s/^version[[:space:]]*=[[:space:]]*\"${CURRENT_VERSION}\"/version    = \"${NEW_VERSION}\"/" "$CARGO_TOML"
rm -f "$CARGO_TOML.bak"
success "Cargo.toml updated."

# ── Update CHANGELOG.md ───────────────────────────────────────────────────────
CHANGELOG="$ROOT/CHANGELOG.md"
TODAY=$(date +%Y-%m-%d)

if grep -q '## \[Unreleased\]' "$CHANGELOG"; then
    # Check if [Unreleased] section has any content
    UNRELEASED_CONTENT=$(awk '/## \[Unreleased\]/{found=1; next} found && /^## \[/{exit} found{print}' "$CHANGELOG" | grep -v '^[[:space:]]*$' || true)

    if [[ -z "$UNRELEASED_CONTENT" ]]; then
        warn "CHANGELOG.md [Unreleased] section is empty — continuing, but add your notes!"
    fi

    # Insert a new [Unreleased] block and rename old one to the new version
    sed -i.bak \
        "s/## \[Unreleased\]/## [Unreleased]\n\n---\n\n## [${NEW_VERSION}] - ${TODAY}/" \
        "$CHANGELOG"
    rm -f "$CHANGELOG.bak"
    success "CHANGELOG.md updated (promoted [Unreleased] → [${NEW_VERSION}] - ${TODAY})."
else
    warn "No [Unreleased] section found in CHANGELOG.md — skipping changelog update."
fi

# ── Run tests ─────────────────────────────────────────────────────────────────
info "Running cargo test --workspace …"
if ! cargo test --workspace 2>&1; then
    die "Tests failed — aborting publish. Fix the failures and re-run."
fi
success "All tests passed."

# ── Commit version bump ───────────────────────────────────────────────────────
git -C "$ROOT" add Cargo.toml Cargo.lock CHANGELOG.md
git -C "$ROOT" commit -m "chore: release v${NEW_VERSION}"
success "Committed version bump."

# ── Publish hooksmith-core ────────────────────────────────────────────────────
info "Publishing hooksmith-core v${NEW_VERSION} …"
cargo publish -p hooksmith-core
success "hooksmith-core published."

# ── Wait for crates.io to index hooksmith-core ────────────────────────────────
info "Waiting for crates.io to index hooksmith-core v${NEW_VERSION} …"
info "(This typically takes 30–90 seconds)"

MAX_WAIT=180   # seconds total
POLL_INTERVAL=15
elapsed=0

while (( elapsed < MAX_WAIT )); do
    sleep $POLL_INTERVAL
    elapsed=$(( elapsed + POLL_INTERVAL ))

    # cargo search returns the latest published version
    INDEXED=$(cargo search hooksmith-core --limit 1 2>/dev/null \
        | grep '^hooksmith-core' \
        | grep -o '"[0-9]*\.[0-9]*\.[0-9]*"' \
        | tr -d '"' || true)

    if [[ "$INDEXED" == "$NEW_VERSION" ]]; then
        success "hooksmith-core v${NEW_VERSION} is now indexed on crates.io."
        break
    fi

    info "  Not yet (found: ${INDEXED:-none}) — waiting ${POLL_INTERVAL}s … (${elapsed}/${MAX_WAIT}s)"
done

if [[ "$INDEXED" != "$NEW_VERSION" ]]; then
    die "hooksmith-core v${NEW_VERSION} was not indexed within ${MAX_WAIT}s.\n  Check https://crates.io/crates/hooksmith-core and re-run the publish steps manually."
fi

# ── Publish discord_hook ──────────────────────────────────────────────────────
info "Publishing discord_hook v${NEW_VERSION} …"
cargo publish -p discord_hook
success "discord_hook published."

# ── Tag and push ──────────────────────────────────────────────────────────────
TAG="v${NEW_VERSION}"
git -C "$ROOT" tag -a "$TAG" -m "Release ${TAG}"
git -C "$ROOT" push origin HEAD
git -C "$ROOT" push origin "$TAG"
success "Pushed commit and tag ${TAG}."

echo ""
success "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
success "  Released hooksmith-core + discord_hook v${NEW_VERSION}"
success "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
