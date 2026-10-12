import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import omarchy.cheatsheets

// Search-only window (`--search`): one field, one flat list ranked across
// every sheet. Ranking lives in Rust (search::rank); this file only renders.
ApplicationWindow {
    id: window

    // Same Tokyo Night palette as Main.qml, and the same size, so the stock
    // `omarchy-cheatsheets` Hyprland window rule fits both windows.
    readonly property color bg: "#1a1b26"
    readonly property color bgDark: "#13141c"
    readonly property color surface: "#24283b"
    readonly property color selection: "#292e42"
    readonly property color muted: "#414868"
    readonly property color fg: "#a9b1d6"
    readonly property color fgBright: "#c0caf5"
    readonly property color dim: "#565f89"
    readonly property color accent: "#7aa2f7"
    readonly property string iconFont: "JetBrainsMono Nerd Font"
    readonly property int hitLimit: 50

    color: Qt.rgba(bg.r, bg.g, bg.b, 0.96)

    visible: true
    width: 980
    height: 660
    minimumWidth: 680
    minimumHeight: 420
    title: "Omarchy Cheatsheets — Search"

    CheatsheetBridge {
        id: bridge
    }

    Shortcut {
        sequence: "Escape"
        onActivated: Qt.quit()
    }

    function refresh() {
        hitList.model = bridge.rankedRows(window.hitLimit)
        hitList.currentIndex = hitList.count > 0 ? 0 : -1
    }

    function moveSelection(step) {
        if (hitList.count === 0) return
        const next = Math.max(0, Math.min(hitList.count - 1, hitList.currentIndex + step))
        hitList.currentIndex = next
        hitList.positionViewAtIndex(next, ListView.Contain)
    }

    Component.onCompleted: {
        // The CLI shim leaves --search in argv; an optional value pre-fills the field.
        const args = Qt.application.arguments
        for (let i = 0; i < args.length; i++) {
            const eq = args[i].match(/^--search=(.*)$/)
            if (eq) {
                searchInput.text = eq[1]
            } else if (args[i] === "--search" && i + 1 < args.length
                       && !args[i + 1].startsWith("--")) {
                searchInput.text = args[i + 1]
            }
        }
        refresh()
        searchInput.forceActiveFocus()
    }

    Connections {
        target: bridge
        function onSearch_textChanged() { window.refresh() }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 18
        spacing: 14

        // Search field: the whole point of this window, so it gets the width.
        Rectangle {
            Layout.fillWidth: true
            height: 52
            radius: 26
            color: window.surface
            border.color: window.accent
            border.width: 2

            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 20
                anchors.rightMargin: 20
                spacing: 12

                Text {
                    text: ""
                    font.family: window.iconFont
                    font.pixelSize: 17
                    color: window.accent
                }

                TextInput {
                    id: searchInput
                    Layout.fillWidth: true
                    color: window.fgBright
                    font.pixelSize: 18
                    clip: true
                    verticalAlignment: TextInput.AlignVCenter
                    onTextChanged: bridge.search_text = text
                    Keys.onUpPressed: window.moveSelection(-1)
                    Keys.onDownPressed: window.moveSelection(1)

                    Text {
                        anchors.fill: parent
                        visible: !searchInput.text
                        text: "Search bindings across all apps…"
                        color: window.dim
                        font.pixelSize: 18
                        verticalAlignment: Text.AlignVCenter
                    }
                }

                Text {
                    visible: searchInput.text.trim().length > 0
                    text: hitList.count + " match" + (hitList.count === 1 ? "" : "es")
                    color: window.dim
                    font.pixelSize: 12
                }

                Rectangle {
                    visible: searchInput.text.length > 0
                    width: 24; height: 24; radius: 12
                    color: clearArea.containsMouse ? window.muted : "transparent"
                    Text {
                        anchors.centerIn: parent
                        text: ""
                        font.family: window.iconFont
                        font.pixelSize: 12
                        color: window.dim
                    }
                    MouseArea {
                        id: clearArea
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: {
                            searchInput.text = ""
                            searchInput.forceActiveFocus()
                        }
                    }
                }
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            ListView {
                id: hitList
                anchors.fill: parent
                clip: true
                spacing: 3
                boundsBehavior: Flickable.StopAtBounds
                model: []

                delegate: Rectangle {
                    required property var modelData
                    required property int index
                    readonly property bool current: hitList.currentIndex === index
                    width: hitList.width
                    height: 46
                    radius: 8
                    color: current ? window.selection : "transparent"

                    Rectangle {
                        visible: current
                        width: 3
                        height: 22
                        radius: 2
                        anchors.left: parent.left
                        anchors.leftMargin: 2
                        anchors.verticalCenter: parent.verticalCenter
                        color: window.accent
                    }

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 14
                        anchors.rightMargin: 14
                        spacing: 12

                        Text {
                            Layout.preferredWidth: 20
                            horizontalAlignment: Text.AlignHCenter
                            text: modelData.icon
                            font.family: window.iconFont
                            font.pixelSize: 15
                            color: current ? window.accent : window.dim
                        }

                        Row {
                            spacing: 5
                            Repeater {
                                model: modelData.keys
                                delegate: Rectangle {
                                    required property string modelData
                                    width: keyText.implicitWidth + 16
                                    height: 26
                                    radius: 6
                                    gradient: Gradient {
                                        GradientStop { position: 0.0; color: window.surface }
                                        GradientStop { position: 1.0; color: window.bgDark }
                                    }
                                    border.color: window.muted
                                    border.width: 1
                                    Text {
                                        id: keyText
                                        anchors.centerIn: parent
                                        text: modelData
                                        color: window.fgBright
                                        font.pixelSize: 13
                                        font.family: window.iconFont
                                    }
                                }
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            text: modelData.description
                            color: current ? window.fgBright : window.fg
                            font.pixelSize: 14
                            elide: Text.ElideRight
                        }

                        Text {
                            text: modelData.sheetTitle + " · " + modelData.section
                            color: window.dim
                            font.pixelSize: 12
                            elide: Text.ElideRight
                            Layout.maximumWidth: hitList.width * 0.3
                        }
                    }

                    MouseArea {
                        anchors.fill: parent
                        hoverEnabled: true
                        onEntered: hitList.currentIndex = index
                    }
                }
            }

            Column {
                anchors.centerIn: parent
                visible: hitList.count === 0
                spacing: 10
                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: ""
                    font.family: window.iconFont
                    font.pixelSize: 28
                    color: window.muted
                }
                Text {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text: searchInput.text.trim().length > 0
                        ? "No bindings match — try fewer words"
                        : "Type what the shortcut does, e.g. \"split pane\" or \"quit\""
                    color: window.dim
                    font.pixelSize: 14
                }
            }
        }

        // Footer hints as mini keycaps
        Row {
            Layout.alignment: Qt.AlignHCenter
            spacing: 18
            Repeater {
                model: [["type", "to search"], ["↑ ↓", "to move"], ["Esc", "to close"]]
                delegate: Row {
                    required property var modelData
                    spacing: 6
                    anchors.verticalCenter: parent.verticalCenter
                    Rectangle {
                        width: hintKey.implicitWidth + 12
                        height: 20
                        radius: 5
                        color: window.surface
                        border.color: window.muted
                        Text {
                            id: hintKey
                            anchors.centerIn: parent
                            text: modelData[0]
                            color: window.fg
                            font.pixelSize: 11
                            font.family: window.iconFont
                        }
                    }
                    Text {
                        text: modelData[1]
                        color: window.dim
                        font.pixelSize: 11
                        anchors.verticalCenter: parent.verticalCenter
                    }
                }
            }
        }
    }
}
