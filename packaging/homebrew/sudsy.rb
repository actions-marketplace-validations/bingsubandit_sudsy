class Sudsy < Formula
  desc "Formatter and linter for C#, JSON, YAML and more"
  homepage "https://github.com/bingsubandit/sudsy"
  version "@VERSION@"
  license "MIT"

  on_macos do
    on_arm do
      url "@URL darwin-arm64@"
      sha256 "@SHA darwin-arm64@"
    end
    on_intel do
      url "@URL darwin-x64@"
      sha256 "@SHA darwin-x64@"
    end
  end

  on_linux do
    on_arm do
      url "@URL linux-arm64@"
      sha256 "@SHA linux-arm64@"
    end
    on_intel do
      url "@URL linux-x64@"
      sha256 "@SHA linux-x64@"
    end
  end

  def install
    libexec.install Dir["*"]
    (bin/"sudsy").write_env_script libexec/"sudsy",
                                   SUDSY_CSHARP_WORKER: libexec/"csharp-worker/Sudsy.Worker.dll"
  end

  def caveats
    "Formatting C# needs the .NET 10 runtime (brew install dotnet)."
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/sudsy --version")
  end
end
