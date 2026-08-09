cask "shelfy" do
  version :latest
  sha256 :no_check

  on_arm do
    depends_on macos: :big_sur
  end
  on_intel do
    depends_on macos: :high_sierra
  end

  # This is a GitHub Release asset, not a repository source zipball.
  url "https://github.com/mcxen/shelfy/releases/latest/download/Shelfy_universal-apple-darwin.app.zip",
      verified: "github.com/mcxen/shelfy/"
  name "Shelfy"
  desc "Desktop file organizer and automation tool"
  homepage "https://github.com/mcxen/shelfy"

  app "Shelfy.app"

  zap trash: [
    "~/Library/Caches/cc.shelfy.app",
    "~/Library/Preferences/cc.shelfy.app.plist",
    "~/Library/Saved Application State/cc.shelfy.app.savedState",
  ]

  caveats <<~EOS
    Shelfy is ad-hoc signed and is not Apple-notarized. If macOS blocks the first
    launch, review the warning and use System Settings → Privacy & Security → Open Anyway.
  EOS
end
