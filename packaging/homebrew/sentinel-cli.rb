class SentinelCli < Formula
  desc "SentinelRS monitoring CLI — deploy and manage your monitoring stack"
  homepage "https://github.com/sentinelrs/sentinelrs"
  version "__VERSION__"
  license "Apache-2.0"

  on_macos do
    url "https://github.com/sentinelrs/sentinelrs/releases/download/v#{version}/sentinel-macos-universal.tar.gz"
    sha256 "__SHA256__"
  end

  on_linux do
    on_intel do
      url "https://github.com/sentinelrs/sentinelrs/releases/download/v#{version}/sentinel-linux-amd64.tar.gz"
      sha256 "__SHA256_LINUX_AMD64__"
    end

    on_arm do
      url "https://github.com/sentinelrs/sentinelrs/releases/download/v#{version}/sentinel-linux-arm64.tar.gz"
      sha256 "__SHA256_LINUX_ARM64__"
    end
  end

  def install
    bin.install "sentinel_cli" => "sentinel"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/sentinel --version")
  end
end
