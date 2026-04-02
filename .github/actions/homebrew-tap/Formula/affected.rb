class Affected < Formula
  desc "Language-agnostic affected package detection and task execution for monorepos"
  homepage "https://github.com/Rani367/affected"
  version "0.2.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/Rani367/affected/releases/download/v#{version}/affected-aarch64-apple-darwin.tar.gz"
      sha256 "d38f68c0df44fd222017a153325798cd166caec1f87ecd7bee9be22e95a1fe34"
    else
      url "https://github.com/Rani367/affected/releases/download/v#{version}/affected-x86_64-apple-darwin.tar.gz"
      sha256 "585f582cb63732bc68c00f0d896ae181e7ed70cc0a1fd07ba16b8b10db67aa0b"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/Rani367/affected/releases/download/v#{version}/affected-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "c4234b7bfd878444d0730e036a779dd3eba3bbe4b1d376b25cb0db3c4b2b78b8"
    else
      url "https://github.com/Rani367/affected/releases/download/v#{version}/affected-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "14d6bc5a82d390088aa48969cbfdcaf4d6e8e9bde5ef02b60c47dae6f7892e35"
    end
  end

  def install
    bin.install "affected"
  end

  test do
    assert_match "affected", shell_output("#{bin}/affected --version")
  end
end
