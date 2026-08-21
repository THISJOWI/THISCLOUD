/* THISCLOUD node configuration form shown as a Calamares view step. */
import io.calamares.core 1.0
import io.calamares.ui 1.0

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: rootItem
    width: parent ? parent.width : 800
    height: parent ? parent.height : 600

    Rectangle {
        anchors.fill: parent
        color: "#0f1115"
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 32
        spacing: 20

        ColumnLayout {
            spacing: 6
            Layout.fillWidth: true

            Label {
                text: qsTr("THISCLOUD Hypervisor Node Configuration")
                font.pointSize: 18
                font.bold: true
                color: "#e6e9ef"
            }

            Label {
                text: qsTr("Specify the cluster role, management IP, and network interface for this hypervisor host.")
                color: "#8b93a3"
                font.pointSize: 11
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: "#2a2f3a"
        }

        GridLayout {
            columns: 2
            columnSpacing: 20
            rowSpacing: 16
            Layout.fillWidth: true

            Label {
                text: qsTr("Node Role:")
                font.bold: true
                color: "#e6e9ef"
                Layout.alignment: Qt.AlignVCenter
            }
            ColumnLayout {
                spacing: 4
                ComboBox {
                    id: roleCombo
                    Layout.preferredWidth: 320
                    model: ["worker", "master"]
                    onCurrentIndexChanged: config.nodeRole = currentText
                    Component.onCompleted: {
                        currentIndex = model.indexOf(config.nodeRole)
                    }
                }
                Label {
                    text: roleCombo.currentText === "master"
                          ? qsTr("Control plane: Runs API server (:8081), daemon (:8080), Web UI (:3000) & cluster state.")
                          : qsTr("Worker node: Dedicated compute and storage hypervisor node.")
                    color: "#8b93a3"
                    font.pointSize: 9
                    Layout.preferredWidth: 320
                    wrapMode: Text.WordWrap
                }
            }

            Label {
                text: qsTr("Cluster Name:")
                font.bold: true
                color: "#e6e9ef"
                Layout.alignment: Qt.AlignVCenter
            }
            TextField {
                id: clusterField
                Layout.preferredWidth: 320
                text: config.clusterName
                placeholderText: "thiscloud"
                onTextEdited: config.clusterName = text
            }

            Label {
                text: qsTr("Management IP:")
                font.bold: true
                color: "#e6e9ef"
                Layout.alignment: Qt.AlignVCenter
            }
            TextField {
                id: ipField
                Layout.preferredWidth: 320
                text: config.nodeIp
                placeholderText: "192.168.1.100"
                onTextEdited: config.nodeIp = text
            }

            Label {
                text: qsTr("Network Interface:")
                font.bold: true
                color: "#e6e9ef"
                Layout.alignment: Qt.AlignVCenter
            }
            ComboBox {
                id: ifaceCombo
                Layout.preferredWidth: 320
                editable: true
                textRole: "text"
                model: ListModel {
                    id: ifaceModel
                    ListElement { text: "eth0" }
                    ListElement { text: "ens3" }
                    ListElement { text: "enp1s0" }
                    ListElement { text: "eno1" }
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