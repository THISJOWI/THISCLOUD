/* THISCLOUD node configuration form shown as a Calamares view step. */
import io.calamares.core 1.0
import io.calamares.ui 1.0

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    width: parent.width
    height: parent.height

    Rectangle {
        anchors.fill: parent
        color: "#0f1115"
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 32
        spacing: 16

        Label {
            text: qsTr("THISCLOUD node configuration")
            font.pointSize: 18
            color: "#e6e9ef"
        }

        Label {
            text: qsTr("Configure how this node joins the THISCLOUD cluster.")
            color: "#e6e9ef"
            Layout.fillWidth: true
            wrapMode: Text.WordWrap
        }

        GridLayout {
            columns: 2
            columnSpacing: 16
            rowSpacing: 12
            Layout.fillWidth: true

            Label { text: qsTr("Node role"); color: "#e6e9ef" }
            ComboBox {
                id: roleCombo
                Layout.preferredWidth: 240
                model: ["worker", "master"]
                onCurrentIndexChanged: config.nodeRole = currentText
                Component.onCompleted: {
                    currentIndex = model.indexOf(config.nodeRole)
                }
            }

            Label { text: qsTr("Cluster name"); color: "#e6e9ef" }
            TextField {
                id: clusterField
                Layout.preferredWidth: 240
                text: config.clusterName
                onTextEdited: config.clusterName = text
            }

            Label { text: qsTr("Node IP address"); color: "#e6e9ef" }
            TextField {
                id: ipField
                Layout.preferredWidth: 240
                text: config.nodeIp
                onTextEdited: config.nodeIp = text
            }

            Label { text: qsTr("Network interface"); color: "#e6e9ef" }
            ComboBox {
                id: ifaceCombo
                Layout.preferredWidth: 240
                editable: true
                textRole: "text"
                model: ListModel {
                    id: ifaceModel
                    ListElement { text: "eth0" }
                    ListElement { text: "ens3" }
                    ListElement { text: "enp1s0" }
                }
                onCurrentTextChanged: config.interface = currentText
                Component.onCompleted: {
                    for (var i = 0; i < ifaceModel.count; ++i) {
                        if (ifaceModel.get(i).text === config.interface) { currentIndex = i; break }
                    }
                }
            }
        }

        Item { Layout.fillHeight: true }
    }

    function onActivate() {}
    function onLeave() {}
}