# THISCLOUD custom view module. This dir is copied into the Calamares
# source tree (src/modules/thiscloudqml/) by build-calamares.sh before cmake.
if(NOT WITH_QML)
    calamares_skip_module( "thiscloudqml (QML is not supported in this build)" )
    return()
endif()

calamares_add_plugin(thiscloudqml
    TYPE viewmodule
    EXPORT_MACRO PLUGINDLLEXPORT_PRO
    SOURCES
        ThisCloudViewStep.cpp
    RESOURCES
        thiscloudqml.qrc
    SHARED_LIB
)