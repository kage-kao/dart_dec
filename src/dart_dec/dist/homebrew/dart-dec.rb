# Homebrew Formula for dart_dec
# Install: brew install dart-dec/tap/dart-dec
# Or: brew tap dart-dec/tap && brew install dart-dec

class DartDec < Formula
  desc "Dart AOT Headless Decompiler — fastest reverse engineering tool for Flutter/Dart apps"
  homepage "https://github.com/dart-dec/dart_dec"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/dart-dec/dart_dec/releases/download/v0.1.0/dart_dec-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_ARM64_MACOS"
    else
      url "https://github.com/dart-dec/dart_dec/releases/download/v0.1.0/dart_dec-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_X64_MACOS"
    end
  end

  on_linux do
    url "https://github.com/dart-dec/dart_dec/releases/download/v0.1.0/dart_dec-x86_64-unknown-linux-gnu.tar.gz"
    sha256 "PLACEHOLDER_SHA256_X64_LINUX"
  end

  def install
    bin.install "dart_dec"

    # Install shell completions
    generate_completions_from_executable(bin/"dart_dec", "completions")

    # Install profiles
    (share/"dart_dec/profiles").install Dir["profiles/*.json"] if Dir.exist?("profiles")
  end

  test do
    assert_match "dart_dec", shell_output("#{bin}/dart_dec --version")
    assert_match "Available", shell_output("#{bin}/dart_dec profiles")
  end
end
