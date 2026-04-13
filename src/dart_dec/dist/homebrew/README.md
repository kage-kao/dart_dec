# Homebrew Tap Setup for dart_dec
#
# To create the tap repository:
#
# 1. Create a GitHub repo: dart-dec/homebrew-tap
#
# 2. Copy dart-dec.rb into the repo as Formula/dart-dec.rb
#
# 3. Update SHA256 hashes after building release binaries:
#    shasum -a 256 dart_dec-*-apple-darwin.tar.gz
#    shasum -a 256 dart_dec-*-linux-gnu.tar.gz
#
# 4. Users install with:
#    brew tap dart-dec/tap
#    brew install dart-dec
#
# CI automation (add to .github/workflows/ci.yml release job):
#
#   - name: Update Homebrew
#     run: |
#       SHA=$(shasum -a 256 dart_dec-*.tar.gz | awk '{print $1}')
#       # Update formula with new SHA and version
