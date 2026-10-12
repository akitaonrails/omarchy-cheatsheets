import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import omarchy.cheatsheets

ApplicationWindow {
    id: window

    // Tokyo Night palette straight from the active Omarchy theme
    // (/usr/share/omarchy/themes/tokyo-night/colors.toml). Sizes are logical
    // pixels: Qt6 Wayland applies the compositor's monitor scale, so fonts
    // follow the system zoom on HiDPI and stay normal on 1x displays.
    readonly property color bg: "#1a1b26"
    readonly property color bgDark: "#13141c"
    readonly property color surface: "#24283b"
    readonly property color selection: "#292e42"
    readonly property color muted: "#414868"
    readonly property color fg: "#a9b1d6"
    readonly property color fgBright: "#c0caf5"
    readonly property color dim: "#565f89"
    readonly property color accent: "#7aa2f7"
    readonly property color cyan: "#449dab"
    readonly property string iconFont: "JetBrainsMono Nerd Font"

    // Slight translucency lets Hyprland's blur bleed through.
    color: Qt.rgba(bg.r, bg.g, bg.b, 0.96)

    visible: true
    width: 980
    height: 660
    minimumWidth: 680
    minimumHeight: 420
    title: "Omarchy Cheatsheets"

    CheatsheetBridge {
        id: bridge
    }

    Shortcut {
        sequence: "Escape"
        onActivated: Qt.quit()
    }

    function refresh() {
        entryList.model = bridge.filteredRows()
        emptyState.visible = entryList.count === 0
    }

    Component.onCompleted: {
        // The CLI shim intentionally leaves --section in argv for us.
        const args = Qt.application.arguments
        for (let i = 0; i < args.length; i++) {
            const eq = args[i].match(/^--section=(.+)$/)
            if (eq) {
                bridge.selectSheet(eq[1])
            } else if (args[i] === "--section" && i + 1 < args.length) {
                bridge.selectSheet(args[i + 1])
            }
        }
        sidebar.icons = ["\uf02d"].concat(listFromQStringList(bridge.sheetIcons()))
        sidebar.model = ["All"].concat(listFromQStringList(bridge.sheetNames()))
        sidebar.titles = ["All sheets"].concat(listFromQStringList(bridge.sheetTitles()))
        sidebar.currentIndex = bridge.current_sheet + 1
        refresh()
        searchInput.forceActiveFocus()
    }

    function listFromQStringList(list) {
        const out = []
        for (let i = 0; i < list.length; i++) out.push(list[i])
        return out
    }

    Connections {
        target: bridge
        function onSearch_textChanged() { window.refresh() }
        function onCurrent_sheetChanged() { window.refresh() }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 18
        spacing: 14

        // Header: title + pill search field
        RowLayout {
            Layout.fillWidth: true
            spacing: 16

            Text {
                text: "Cheatsheets"
                color: window.fgBright
                font.pixelSize: 20
                font.bold: true
            }

            Rectangle {
                Layout.fillWidth: true
                height: 42
                radius: 21
                color: window.surface
                border.color: searchInput.activeFocus ? window.accent : window.muted
                border.width: searchInput.activeFocus ? 2 : 1

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 16
                    anchors.rightMargin: 16
                    spacing: 10

                    Text {
                        text: "\uf002"
                        font.family: window.iconFont
                        font.pixelSize: 14
                        color: searchInput.activeFocus ? window.accent : window.dim
                    }

                    TextInput {
                        id: searchInput
                        Layout.fillWidth: true
                        color: window.fgBright
                        font.pixelSize: 15
                        clip: true
                        verticalAlignment: TextInput.AlignVCenter
                        onTextChanged: bridge.search_text = text

                        Text {
                            anchors.fill: parent
                            anchors.verticalCenter: parent.verticalCenter
                            visible: !searchInput.text
                            text: "Type to filter bindings…"
                            color: window.dim
                            font.pixelSize: 15
                            verticalAlignment: Text.AlignVCenter
                        }
                    }

                    Text {
                        visible: searchInput.text.length > 0
                        text: entryList.count + " match" + (entryList.count === 1 ? "" : "es")
                        color: window.dim
                        font.pixelSize: 12
                    }

                    Rectangle {
                        visible: searchInput.text.length > 0
                        width: 22; height: 22; radius: 11
                        color: clearArea.containsMouse ? window.muted : "transparent"
                        Text {
                            anchors.centerIn: parent
                            text: "\uf00d"
                            font.family: window.iconFont
                            font.pixelSize: 11
                            color: window.dim
                        }
                        MouseArea {
                            id: clearArea
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: searchInput.text = ""
                        }
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 14

            // Sidebar
            ListView {
                id: sidebar
                Layout.fillHeight: true
                Layout.preferredWidth: 210
                spacing: 4
                clip: true
                property var titles: []
                property var icons: []
                model: []

                delegate: Rectangle {
                    required property int index
                    required property string modelData
                    width: sidebar.width
                    height: 44
                    radius: 8
                    color: sidebar.currentIndex === index ? window.selection
                         : (hoverArea.containsMouse ? window.surface : "transparent")

                    Rectangle {
                        visible: sidebar.currentIndex === index
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
                        spacing: 10

                        Text {
                            text: sidebar.icons[index] ?? ""
                            font.family: window.iconFont
                            font.pixelSize: 15
                            color: sidebar.currentIndex === index ? window.accent : window.dim
                        }
                        Text {
                            Layout.fillWidth: true
                            text: sidebar.titles[index] ?? modelData
                            color: sidebar.currentIndex === index ? window.fgBright : window.fg
                            font.pixelSize: 14
                            elide: Text.ElideRight
                        }
                    }
                    MouseArea {
                        id: hoverArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            sidebar.currentIndex = index
                            bridge.selectSheet(index === 0 ? "" : sidebar.model[index])
                        }
                    }
                }
            }

            Rectangle { Layout.fillHeight: true; width: 1; color: window.surface }

            // Entries
            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                ListView {
                    id: entryList
                    anchors.fill: parent
                    clip: true
                    spacing: 3
                    boundsBehavior: Flickable.StopAtBounds
                    model: []

                    section.property: "group"
                    section.criteria: ViewSection.FullString
                    section.delegate: Item {
                        required property string section
                        width: entryList.width
                        height: 42
                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 6
                            anchors.rightMargin: 6
                            anchors.topMargin: 10
                            spacing: 10
                            Text {
                                text: section
                                color: window.accent
                                font.pixelSize: 12
                                font.bold: true
                                font.letterSpacing: 1.6
                                font.capitalization: Font.AllUppercase
                            }
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 1
                                color: window.surface
                            }
                        }
                    }

                    delegate: Rectangle {
                        required property var modelData
                        required property int index
                        width: entryList.width
                        height: 40
                        radius: 8
                        color: rowHover.containsMouse ? window.selection : "transparent"

                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 12
                            anchors.rightMargin: 12
                            spacing: 12

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
                                color: window.fg
                                font.pixelSize: 14
                                elide: Text.ElideRight
                            }
                        }

                        MouseArea {
                            id: rowHover
                            anchors.fill: parent
                            hoverEnabled: true
                        }
                    }
                }

                Column {
                    id: emptyState
                    anchors.centerIn: parent
                    visible: false
                    spacing: 10
                    Text {
                        anchors.horizontalCenter: parent.horizontalCenter
                        text: "\uf002"
                        font.family: window.iconFont
                        font.pixelSize: 28
                        color: window.muted
                    }
                    Text {
                        text: "No bindings match — try a shorter query"
                        color: window.dim
                        font.pixelSize: 14
                    }
                }
            }
        }

        // Footer hints as mini keycaps
        Row {
            Layout.alignment: Qt.AlignHCenter
            spacing: 18
            Repeater {
                model: [["type", "to filter"], ["click", "to scope"], ["Esc", "to close"]]
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
