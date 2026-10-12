import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import omarchy.cheatsheets

ApplicationWindow {
    id: window

    // Palette from the chosen (default: active) Omarchy theme's colors.toml,
    // resolved in Rust (settings.rs). Sizes are logical pixels: Qt6 Wayland
    // applies the compositor's monitor scale, so fonts follow the system zoom
    // on HiDPI and stay normal on 1x displays.
    property var pal: appSettings.colors()
    readonly property color bg: pal.bg
    readonly property color bgDark: pal.bgDark
    readonly property color surface: pal.surface
    readonly property color selection: pal.selection
    readonly property color muted: pal.muted
    readonly property color fg: pal.fg
    readonly property color fgBright: pal.fgBright
    readonly property color dim: pal.dim
    readonly property color accent: pal.accent
    readonly property color cyan: pal.cyan
    readonly property string iconFont: "JetBrainsMono Nerd Font"
    readonly property string textFont: appSettings.font_family || Qt.application.font.family

    // The layout was designed at 14px (settings::FONT_SIZE_DEFAULT); every
    // size scales with the chosen font so rows never clip at large sizes.
    readonly property int baseFontSize: 14
    readonly property real k: appSettings.font_size / baseFontSize
    function px(n) { return Math.round(n * k) }

    // Slight translucency (configurable) lets Hyprland's blur bleed through.
    color: Qt.rgba(bg.r, bg.g, bg.b, appSettings.opacity)

    visible: true
    width: 980
    height: 660
    minimumWidth: 680
    minimumHeight: 420
    title: "Omarchy Cheatsheets"

    CheatsheetBridge {
        id: bridge
    }

    AppSettings {
        id: appSettings
        onThemeChanged: window.pal = colors()
    }

    // Esc closes the settings panel first, the app only when nothing is open.
    Shortcut {
        sequence: "Escape"
        onActivated: settingsPanel.opened ? settingsPanel.close() : Qt.quit()
    }
    Shortcut {
        sequence: "Ctrl+,"
        onActivated: settingsPanel.opened ? settingsPanel.close() : settingsPanel.open()
    }
    Shortcut {
        sequences: ["Ctrl+=", "Ctrl++"]
        onActivated: appSettings.applyFontSize(appSettings.font_size + 1)
    }
    Shortcut {
        sequence: "Ctrl+-"
        onActivated: appSettings.applyFontSize(appSettings.font_size - 1)
    }
    Shortcut {
        sequence: "Ctrl+0"
        onActivated: appSettings.applyFontSize(window.baseFontSize)
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

    // Appearance settings. Every change is applied live and saved by Rust
    // (settings_bridge.rs); this panel only renders the current values.
    Popup {
        id: settingsPanel
        x: window.width - width - 18
        y: window.px(42) + 28
        width: Math.max(320, window.px(340))
        padding: 18
        // Esc is handled by the window Shortcut so it never quits the app
        // while the panel is open.
        closePolicy: Popup.CloseOnPressOutside
        // Keyboard-first like the rest of the app: Tab walks the controls,
        // arrows change the focused one.
        focus: true
        onOpened: themeBox.forceActiveFocus()
        onClosed: searchInput.forceActiveFocus()

        // Fusion controls take their colors from this palette.
        palette.window: window.surface
        palette.windowText: window.fg
        palette.base: window.bg
        palette.alternateBase: window.selection
        palette.text: window.fgBright
        palette.button: window.surface
        palette.buttonText: window.fgBright
        palette.highlight: window.accent
        palette.highlightedText: window.bg
        palette.mid: window.muted
        palette.dark: window.bgDark
        palette.light: window.selection
        palette.placeholderText: window.dim

        background: Rectangle {
            radius: 12
            color: window.surface
            border.color: window.muted
            border.width: 1
        }

        contentItem: ColumnLayout {
            spacing: 14

            Text {
                text: "Appearance"
                color: window.fgBright
                font.pixelSize: window.px(16)
                font.family: window.textFont
                font.bold: true
            }

            Text {
                text: "Theme"
                color: window.dim
                font.pixelSize: window.px(12)
                font.family: window.textFont
            }
            ComboBox {
                id: themeBox
                Layout.fillWidth: true
                textRole: "text"
                valueRole: "value"
                model: appSettings.themeNames().map(name => ({
                    value: name,
                    text: name === "omarchy" ? "Omarchy (follow system)" : name
                }))
                currentIndex: Math.max(0, indexOfValue(appSettings.theme))
                onActivated: appSettings.applyTheme(currentValue)
            }

            Text {
                text: "Font"
                color: window.dim
                font.pixelSize: window.px(12)
                font.family: window.textFont
            }
            ComboBox {
                id: fontBox
                Layout.fillWidth: true
                textRole: "text"
                valueRole: "value"
                model: [{ value: "", text: "System default" }].concat(
                    Qt.fontFamilies().map(family => ({ value: family, text: family })))
                currentIndex: Math.max(0, indexOfValue(appSettings.font_family))
                onActivated: appSettings.applyFontFamily(currentValue)
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 12
                Text {
                    Layout.fillWidth: true
                    text: "Font size"
                    color: window.dim
                    font.pixelSize: window.px(12)
                    font.family: window.textFont
                }
                SpinBox {
                    from: 10
                    to: 24
                    value: appSettings.font_size
                    onValueModified: appSettings.applyFontSize(value)
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 12
                Text {
                    text: "Opacity"
                    color: window.dim
                    font.pixelSize: window.px(12)
                    font.family: window.textFont
                }
                Slider {
                    Layout.fillWidth: true
                    from: 50
                    to: 100
                    stepSize: 1
                    value: Math.round(appSettings.opacity * 100)
                    onMoved: appSettings.applyOpacity(value / 100)
                }
                Text {
                    text: Math.round(appSettings.opacity * 100) + "%"
                    color: window.fg
                    font.pixelSize: window.px(12)
                    font.family: window.textFont
                }
            }

            Button {
                Layout.alignment: Qt.AlignRight
                text: "Reset to defaults"
                onClicked: appSettings.reset()
            }
        }
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
                font.pixelSize: window.px(20)
                font.family: window.textFont
                font.bold: true
            }

            Rectangle {
                Layout.fillWidth: true
                // Layout-managed: a plain `height:` binding is overwritten by
                // the layout and would not follow later font-size changes.
                Layout.preferredHeight: window.px(42)
                radius: height / 2
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
                        font.pixelSize: window.px(14)
                        color: searchInput.activeFocus ? window.accent : window.dim
                    }

                    TextInput {
                        id: searchInput
                        Layout.fillWidth: true
                        color: window.fgBright
                        font.pixelSize: window.px(15)
                        font.family: window.textFont
                        clip: true
                        verticalAlignment: TextInput.AlignVCenter
                        onTextChanged: bridge.search_text = text

                        Text {
                            anchors.fill: parent
                            anchors.verticalCenter: parent.verticalCenter
                            visible: !searchInput.text
                            text: "Type to filter bindings…"
                            color: window.dim
                            font.pixelSize: window.px(15)
                            font.family: window.textFont
                            verticalAlignment: Text.AlignVCenter
                        }
                    }

                    Text {
                        visible: searchInput.text.length > 0
                        text: entryList.count + " match" + (entryList.count === 1 ? "" : "es")
                        color: window.dim
                        font.pixelSize: window.px(12)
                        font.family: window.textFont
                    }

                    Rectangle {
                        visible: searchInput.text.length > 0
                        implicitWidth: window.px(22); implicitHeight: implicitWidth; radius: width / 2
                        color: clearArea.containsMouse ? window.muted : "transparent"
                        Text {
                            anchors.centerIn: parent
                            text: "\uf00d"
                            font.family: window.iconFont
                            font.pixelSize: window.px(11)
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

            Rectangle {
                implicitWidth: window.px(42); implicitHeight: implicitWidth; radius: width / 2
                color: settingsPanel.opened || gearArea.containsMouse ? window.surface : "transparent"
                border.color: settingsPanel.opened ? window.accent : window.muted
                border.width: 1
                Text {
                    anchors.centerIn: parent
                    text: "\uf013"
                    font.family: window.iconFont
                    font.pixelSize: window.px(15)
                    color: settingsPanel.opened ? window.accent : window.dim
                }
                MouseArea {
                    id: gearArea
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: settingsPanel.opened ? settingsPanel.close() : settingsPanel.open()
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
                Layout.preferredWidth: window.px(210)
                spacing: 4
                clip: true
                property var titles: []
                property var icons: []
                model: []

                delegate: Rectangle {
                    required property int index
                    required property string modelData
                    width: sidebar.width
                    height: window.px(44)
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
                            font.pixelSize: window.px(15)
                            color: sidebar.currentIndex === index ? window.accent : window.dim
                        }
                        Text {
                            Layout.fillWidth: true
                            text: sidebar.titles[index] ?? modelData
                            color: sidebar.currentIndex === index ? window.fgBright : window.fg
                            font.pixelSize: window.px(14)
                            font.family: window.textFont
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
                        height: window.px(42)
                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 6
                            anchors.rightMargin: 6
                            anchors.topMargin: 10
                            spacing: 10
                            Text {
                                text: section
                                color: window.accent
                                font.pixelSize: window.px(12)
                                font.family: window.textFont
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
                        height: window.px(40)
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
                                        height: window.px(26)
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
                                            font.pixelSize: window.px(13)
                                            font.family: window.iconFont
                                        }
                                    }
                                }
                            }

                            Text {
                                Layout.fillWidth: true
                                text: modelData.description
                                color: window.fg
                                font.pixelSize: window.px(14)
                                font.family: window.textFont
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
                        font.pixelSize: window.px(28)
                        color: window.muted
                    }
                    Text {
                        text: "No bindings match — try a shorter query"
                        color: window.dim
                        font.pixelSize: window.px(14)
                        font.family: window.textFont
                    }
                }
            }
        }

        // Footer hints as mini keycaps
        Row {
            Layout.alignment: Qt.AlignHCenter
            spacing: 18
            Repeater {
                model: [["type", "to filter"], ["click", "to scope"], ["Ctrl+,", "settings"], ["Esc", "to close"]]
                delegate: Row {
                    required property var modelData
                    spacing: 6
                    anchors.verticalCenter: parent.verticalCenter
                    Rectangle {
                        width: hintKey.implicitWidth + 12
                        height: window.px(20)
                        radius: 5
                        color: window.surface
                        border.color: window.muted
                        Text {
                            id: hintKey
                            anchors.centerIn: parent
                            text: modelData[0]
                            color: window.fg
                            font.pixelSize: window.px(11)
                            font.family: window.iconFont
                        }
                    }
                    Text {
                        text: modelData[1]
                        color: window.dim
                        font.pixelSize: window.px(11)
                        font.family: window.textFont
                        anchors.verticalCenter: parent.verticalCenter
                    }
                }
            }
        }
    }
}
