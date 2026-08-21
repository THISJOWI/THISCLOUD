import QtQuick 2.15
import io.calamares.core 1.0
import io.calamares.ui 1.0

Presentation
{
    id: presentation

    Timer
    {
        interval: 6000
        running: true
        repeat: true
        onTriggered: presentation.goToNextSlide()
    }

    Slide
    {
        anchors.fill: parent

        Image
        {
            id: slide1
            anchors.fill: parent
            source: "slide-1.png"
            fillMode: Image.PreserveAspectCrop
        }
    }

    Slide
    {
        anchors.fill: parent

        Image
        {
            anchors.fill: parent
            source: "slide-2.png"
            fillMode: Image.PreserveAspectCrop
        }
    }

    Slide
    {
        anchors.fill: parent

        Image
        {
            anchors.fill: parent
            source: "slide-3.png"
            fillMode: Image.PreserveAspectCrop
        }
    }

    Slide
    {
        anchors.fill: parent

        Image
        {
            anchors.fill: parent
            source: "slide-4.png"
            fillMode: Image.PreserveAspectCrop
        }
    }

    onActivate: {}
    onLeave: {}
}