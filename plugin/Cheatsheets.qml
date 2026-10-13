import QtQuick
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui

// Bar launcher for the omarchy-cheatsheets app. The plugin itself carries no
// cheatsheet UI — the app is a standalone binary (AUR: omarchy-cheatsheets /
// omarchy-cheatsheets-bin). If the binary is missing, clicking offers the
// install instead of failing silently. Left click opens the search-only
// window, right click the full browser.
BarWidget {
  id: root
  moduleName: "akitaonrails.cheatsheets"

  property bool installed: false

  function refresh() {
    if (!checkProc.running) checkProc.running = true
  }

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  Process {
    id: checkProc
    command: ["sh", "-c", "command -v omarchy-cheatsheets"]
    onExited: function(exitCode) { root.installed = exitCode === 0 }
  }

  Timer {
    interval: 60000
    running: true
    repeat: true
    triggeredOnStart: true
    onTriggered: root.refresh()
  }

  BarIconButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: "\uf02d"
    slotSize: Style.bar.statusSlot
    fontSize: Style.font.caption
    tooltipText: root.installed
      ? "Search cheatsheets — right-click to browse"
      : "Cheatsheets — click to install from the AUR"
    // Left click goes straight to search (find a binding by what it does);
    // right click opens the full sidebar browser. The plugin updates apart
    // from the AUR binary, so an older binary that rejects --search (clap
    // usage error, exit 2) falls back to the browser instead of doing nothing.
    onPressed: function(button) {
      if (root.installed) {
        root.bar.run(button === Qt.RightButton
          ? "omarchy-cheatsheets"
          : "omarchy-cheatsheets --search; [ $? -eq 2 ] && omarchy-cheatsheets")
      } else {
        root.bar.run("omarchy-launch-floating-terminal-with-presentation omarchy pkg aur add omarchy-cheatsheets-bin")
      }
    }
  }
}
