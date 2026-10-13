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
    // Premultiplied in Rust (Palette::window_color): Qt.rgba here would wash
    // the window out toward white instead of making it see-through.
    color: appSettings.window_color

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

    // Tab walks the sheet list; the search field keeps focus throughout.
    function cycleSheet(step) {
        const count = sidebar.count
        if (count < 2) return
        const next = (sidebar.currentIndex + step + count) % count
        sidebar.currentIndex = next
        bridge.selectSheet(next === 0 ? "" : sidebar.model[next])
    }

    Shortcut {
        sequence: "Tab"
        onActivated: window.cycleSheet(1)
    }
    Shortcut {
        sequence: "Shift+Tab"
        onActivated: window.cycleSheet(-1)
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
        onActivated: appSettings.applyFontSize(appSettings.font_size_default)
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

    // Small building blocks shared by the list and the appearance drawer.
    component Keycap: Rectangle {
        property string label
        width: capText.implicitWidth + 16
        height: window.px(26)
        radius: 6
        gradient: Gradient {
            GradientStop { position: 0.0; color: window.surface }
            GradientStop { position: 1.0; color: window.bgDark }
        }
        border.color: window.muted
        border.width: 1
        Text {
            id: capText
            anchors.centerIn: parent
            text: parent.label
            color: window.fgBright
            font.pixelSize: window.px(13)
            font.family: window.iconFont
        }
    }

    component SectionLabel: Text {
        color: window.accent
        font.pixelSize: window.px(11)
        font.family: window.textFont
        font.bold: true
        font.letterSpacing: 1.6
        font.capitalization: Font.AllUppercase
    }

    component KeyButton: Rectangle {
        id: keyButton
        property string glyph
        signal activated()
        implicitWidth: window.px(30)
        implicitHeight: implicitWidth
        radius: 6
        opacity: enabled ? 1 : 0.35
        gradient: Gradient {
            GradientStop { position: 0.0; color: keyArea.containsMouse ? window.selection : window.surface }
            GradientStop { position: 1.0; color: window.bgDark }
        }
        border.color: keyArea.containsMouse ? window.accent : window.muted
        border.width: 1
        Text {
            anchors.centerIn: parent
            text: keyButton.glyph
            color: window.fgBright
            font.pixelSize: window.px(12)
            font.family: window.iconFont
        }
        MouseArea {
            id: keyArea
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: keyButton.activated()
        }
    }

    // Appearance drawer. Every change is applied live and saved by Rust
    // (settings_bridge.rs); this panel only renders the current values.
    Popup {
        id: settingsPanel
        x: window.width - width
        y: 0
        width: Math.min(window.width - 40, Math.max(360, window.px(400)))
        height: window.height
        padding: 0
        // Non-modal on purpose: a modal popup blocks the window Shortcuts,
        // and Esc must close the drawer, not be swallowed.
        modal: false
        dim: true
        closePolicy: Popup.CloseOnPressOutside
        focus: true
        // Rebuilt on every open so a theme switched in Omarchy meanwhile
        // shows its current colors on the "Follow Omarchy" card.
        property var choices: []
        onAboutToShow: choices = appSettings.themeChoices()
        onOpened: themeGrid.forceActiveFocus()
        onClosed: searchInput.forceActiveFocus()

        enter: Transition {
            NumberAnimation { property: "x"; from: window.width; to: window.width - settingsPanel.width; duration: 160; easing.type: Easing.OutCubic }
        }
        exit: Transition {
            NumberAnimation { property: "x"; to: window.width; duration: 120; easing.type: Easing.InCubic }
        }

        Overlay.modeless: Rectangle { color: Qt.rgba(0, 0, 0, 0.35) }

        background: Rectangle {
            color: window.bg
            Rectangle { width: 1; height: parent.height; color: window.muted }
        }

        contentItem: ColumnLayout {
            spacing: window.px(14)

            // Header
            RowLayout {
                Layout.fillWidth: true
                Layout.topMargin: 20
                Layout.leftMargin: 22
                Layout.rightMargin: 18
                spacing: 10
                Text {
                    text: ""
                    font.family: window.iconFont
                    font.pixelSize: window.px(16)
                    color: window.accent
                }
                Text {
                    Layout.fillWidth: true
                    text: "Appearance"
                    color: window.fgBright
                    font.pixelSize: window.px(18)
                    font.family: window.textFont
                    font.bold: true
                }
                Rectangle {
                    implicitWidth: window.px(28); implicitHeight: implicitWidth; radius: width / 2
                    color: closeArea.containsMouse ? window.surface : "transparent"
                    Text {
                        anchors.centerIn: parent
                        text: ""
                        font.family: window.iconFont
                        font.pixelSize: window.px(12)
                        color: closeArea.containsMouse ? window.fgBright : window.dim
                    }
                    MouseArea {
                        id: closeArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: settingsPanel.close()
                    }
                }
            }

            // Live preview: a real row rendered with the current choices.
            Rectangle {
                Layout.fillWidth: true
                Layout.leftMargin: 18
                Layout.rightMargin: 18
                implicitHeight: window.px(58)
                radius: 10
                color: window.surface
                border.color: window.muted
                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 14
                    anchors.rightMargin: 14
                    spacing: 12
                    Row {
                        spacing: 5
                        Keycap { label: "Space" }
                        Keycap { label: "s" }
                        Keycap { label: "g" }
                    }
                    Text {
                        Layout.fillWidth: true
                        text: "Grep in project"
                        color: window.fg
                        font.pixelSize: window.px(14)
                        font.family: window.textFont
                        elide: Text.ElideRight
                    }
                }
            }

            // Text: font family + size
            ColumnLayout {
                Layout.fillWidth: true
                Layout.topMargin: 4
                Layout.leftMargin: 22
                Layout.rightMargin: 22
                spacing: 8
                SectionLabel { text: "Text" }

                ComboBox {
                    id: fontBox
                    Layout.fillWidth: true
                    implicitHeight: window.px(36)
                    textRole: "text"
                    valueRole: "value"
                    model: [{ value: "", text: "System default" }].concat(
                        Qt.fontFamilies().map(family => ({ value: family, text: family })))
                    currentIndex: Math.max(0, indexOfValue(appSettings.font_family))
                    onActivated: appSettings.applyFontFamily(currentValue)

                    background: Rectangle {
                        radius: height / 2
                        color: window.surface
                        border.color: fontBox.activeFocus || fontBox.popup.visible ? window.accent : window.muted
                        border.width: fontBox.activeFocus ? 2 : 1
                    }
                    contentItem: Text {
                        leftPadding: 16
                        rightPadding: 8
                        text: fontBox.displayText
                        color: window.fgBright
                        font.pixelSize: window.px(13)
                        font.family: fontBox.currentValue || Qt.application.font.family
                        verticalAlignment: Text.AlignVCenter
                        elide: Text.ElideRight
                    }
                    indicator: Text {
                        x: fontBox.width - width - 16
                        y: (fontBox.height - height) / 2
                        text: ""
                        font.family: window.iconFont
                        font.pixelSize: window.px(10)
                        color: window.dim
                    }
                    delegate: ItemDelegate {
                        required property var modelData
                        required property int index
                        width: fontBox.width - 8
                        height: window.px(30)
                        highlighted: fontBox.highlightedIndex === index
                        contentItem: Text {
                            text: modelData.text
                            color: highlighted ? window.fgBright : window.fg
                            font.pixelSize: window.px(13)
                            // Each family previews itself.
                            font.family: modelData.value || Qt.application.font.family
                            verticalAlignment: Text.AlignVCenter
                            elide: Text.ElideRight
                        }
                        background: Rectangle {
                            radius: 6
                            color: highlighted ? window.selection : "transparent"
                        }
                    }
                    popup: Popup {
                        y: fontBox.height + 4
                        width: fontBox.width
                        height: Math.min(contentItem.implicitHeight + 8, window.px(280))
                        padding: 4
                        contentItem: ListView {
                            clip: true
                            implicitHeight: contentHeight
                            model: fontBox.popup.visible ? fontBox.delegateModel : null
                            currentIndex: fontBox.highlightedIndex
                            ScrollIndicator.vertical: ScrollIndicator {}
                        }
                        background: Rectangle {
                            radius: 10
                            color: window.surface
                            border.color: window.muted
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 8
                    Text {
                        Layout.fillWidth: true
                        text: "Size"
                        color: window.fg
                        font.pixelSize: window.px(13)
                        font.family: window.textFont
                    }
                    KeyButton {
                        glyph: ""
                        enabled: appSettings.font_size > appSettings.font_size_min
                        onActivated: appSettings.applyFontSize(appSettings.font_size - 1)
                    }
                    Text {
                        Layout.preferredWidth: window.px(54)
                        horizontalAlignment: Text.AlignHCenter
                        text: appSettings.font_size + " px"
                        color: window.fgBright
                        font.pixelSize: window.px(13)
                        font.family: window.textFont
                    }
                    KeyButton {
                        glyph: ""
                        enabled: appSettings.font_size < appSettings.font_size_max
                        onActivated: appSettings.applyFontSize(appSettings.font_size + 1)
                    }
                }
            }

            // Window: opacity
            ColumnLayout {
                Layout.fillWidth: true
                Layout.topMargin: 4
                Layout.leftMargin: 22
                Layout.rightMargin: 22
                spacing: 4
                SectionLabel { text: "Window" }
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 12
                    Text {
                        text: "Opacity"
                        color: window.fg
                        font.pixelSize: window.px(13)
                        font.family: window.textFont
                    }
                    Slider {
                        id: opacitySlider
                        Layout.fillWidth: true
                        from: Math.round(appSettings.opacity_min * 100)
                        to: 100
                        stepSize: 1
                        value: Math.round(appSettings.opacity * 100)
                        onMoved: appSettings.applyOpacity(value / 100)
                        // Implicit sizes are what give the Slider its height:
                        // without them the layout makes it 0px tall and the
                        // handle is drawn but can't be grabbed.
                        background: Rectangle {
                            x: opacitySlider.leftPadding
                            y: opacitySlider.topPadding + opacitySlider.availableHeight / 2 - height / 2
                            implicitWidth: window.px(160)
                            implicitHeight: 4
                            width: opacitySlider.availableWidth
                            height: implicitHeight
                            radius: 2
                            color: window.surface
                            Rectangle {
                                width: opacitySlider.visualPosition * parent.width
                                height: parent.height
                                radius: 2
                                color: window.accent
                            }
                        }
                        handle: Rectangle {
                            x: opacitySlider.leftPadding + opacitySlider.visualPosition * (opacitySlider.availableWidth - width)
                            y: opacitySlider.topPadding + opacitySlider.availableHeight / 2 - height / 2
                            implicitWidth: window.px(16)
                            implicitHeight: implicitWidth
                            radius: width / 2
                            color: opacitySlider.pressed ? window.fgBright : window.accent
                            border.color: window.bg
                            border.width: 2
                        }
                    }
                    Text {
                        Layout.preferredWidth: window.px(40)
                        horizontalAlignment: Text.AlignRight
                        text: Math.round(appSettings.opacity * 100) + "%"
                        color: window.fgBright
                        font.pixelSize: window.px(13)
                        font.family: window.textFont
                    }
                }
            }

            // Theme cards: each previews its own palette; arrows switch live.
            SectionLabel {
                Layout.leftMargin: 22
                Layout.topMargin: 4
                text: "Theme"
            }
            GridView {
                id: themeGrid
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.leftMargin: 16
                Layout.rightMargin: 16
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                keyNavigationEnabled: true
                cellWidth: Math.floor(width / 3)
                cellHeight: window.px(72)
                model: settingsPanel.choices
                currentIndex: Math.max(0, settingsPanel.choices.findIndex(c => c.name === appSettings.theme))
                onCurrentIndexChanged: {
                    const choice = settingsPanel.choices[currentIndex]
                    if (activeFocus && choice && choice.name !== appSettings.theme)
                        appSettings.applyTheme(choice.name)
                }
                ScrollIndicator.vertical: ScrollIndicator {}

                delegate: Item {
                    id: card
                    required property var modelData
                    required property int index
                    readonly property var c: modelData.colors
                    readonly property bool selected: modelData.name === appSettings.theme
                    width: themeGrid.cellWidth
                    height: themeGrid.cellHeight

                    Rectangle {
                        anchors.fill: parent
                        anchors.margins: 4
                        radius: 10
                        color: card.c.bg
                        border.width: card.selected ? 2 : 1
                        border.color: card.selected ? window.accent
                                    : (cardArea.containsMouse ? window.dim : card.c.muted)

                        Column {
                            anchors.fill: parent
                            anchors.margins: 9
                            spacing: 5
                            Row {
                                spacing: 4
                                Repeater {
                                    model: [card.c.accent, card.c.cyan, card.c.fg, card.c.selection]
                                    delegate: Rectangle {
                                        required property string modelData
                                        width: window.px(11)
                                        height: width
                                        radius: width / 2
                                        color: modelData
                                    }
                                }
                            }
                            Text {
                                width: parent.width
                                text: card.modelData.label
                                color: card.c.fgBright
                                font.pixelSize: window.px(11)
                                font.family: window.textFont
                                font.bold: true
                                elide: Text.ElideRight
                            }
                            Text {
                                width: parent.width
                                visible: text.length > 0
                                text: card.modelData.detail
                                color: card.c.dim
                                font.pixelSize: window.px(10)
                                font.family: window.textFont
                                elide: Text.ElideRight
                            }
                        }

                        Text {
                            visible: card.selected
                            anchors.top: parent.top
                            anchors.right: parent.right
                            anchors.margins: 8
                            text: ""
                            font.family: window.iconFont
                            font.pixelSize: window.px(11)
                            color: card.c.accent
                        }

                        MouseArea {
                            id: cardArea
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: appSettings.applyTheme(card.modelData.name)
                        }
                    }
                }
            }

            // Footer: where it is saved + reset
            Rectangle { Layout.fillWidth: true; implicitHeight: 1; color: window.surface }
            RowLayout {
                Layout.fillWidth: true
                Layout.leftMargin: 22
                Layout.rightMargin: 18
                Layout.bottomMargin: 16
                spacing: 10
                Text {
                    Layout.fillWidth: true
                    text: "Saved to " + appSettings.settingsPath()
                    color: window.dim
                    font.pixelSize: window.px(10)
                    font.family: window.textFont
                    elide: Text.ElideMiddle
                }
                Rectangle {
                    implicitWidth: resetRow.implicitWidth + 24
                    implicitHeight: window.px(28)
                    radius: height / 2
                    color: resetArea.containsMouse ? window.surface : "transparent"
                    border.color: resetArea.containsMouse ? window.accent : window.muted
                    Row {
                        id: resetRow
                        anchors.centerIn: parent
                        spacing: 6
                        Text {
                            text: ""
                            font.family: window.iconFont
                            font.pixelSize: window.px(11)
                            color: window.fg
                            anchors.verticalCenter: parent.verticalCenter
                        }
                        Text {
                            text: "Reset"
                            color: window.fg
                            font.pixelSize: window.px(12)
                            font.family: window.textFont
                            anchors.verticalCenter: parent.verticalCenter
                        }
                    }
                    MouseArea {
                        id: resetArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: appSettings.reset()
                    }
                }
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
                                    delegate: Keycap {
                                        required property string modelData
                                        label: modelData
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
                model: [["type", "to filter"], ["Tab", "next app"], ["click", "to scope"], ["Ctrl+,", "settings"], ["Esc", "to close"]]
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
