class Localcode < Formula
  desc "AI-powered coding assistant for the terminal"
  homepage "https://github.com/Svetozar-Technologies/LocalCode"
  version "0.4.1"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/Svetozar-Technologies/LocalCode/releases/download/v#{version}/localcode-darwin-arm64.tar.gz"
      sha256 "PLACEHOLDER"
    end
    on_intel do
      url "https://github.com/Svetozar-Technologies/LocalCode/releases/download/v#{version}/localcode-darwin-x64.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/Svetozar-Technologies/LocalCode/releases/download/v#{version}/localcode-linux-arm64.tar.gz"
      sha256 "PLACEHOLDER"
    end
    on_intel do
      url "https://github.com/Svetozar-Technologies/LocalCode/releases/download/v#{version}/localcode-linux-x64.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "localcode"
  end

  test do
    assert_match "LocalCode", shell_output("#{bin}/localcode --version")
  end
end
