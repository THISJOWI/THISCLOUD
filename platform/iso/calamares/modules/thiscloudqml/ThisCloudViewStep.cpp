/* THISCLOUD install-config view step. */
#include "ThisCloudViewStep.h"

#include "GlobalStorage.h"
#include "utils/CalamaresUtilsGui.h"
#include "utils/Logger.h"
#include "utils/Variant.h"

#include <QVariant>

ThisCloudConfig::ThisCloudConfig( QObject* parent )
    : QObject( parent )
{
}

ThisCloudViewStep::ThisCloudViewStep( QObject* parent )
    : Calamares::QmlViewStep( parent )
    , m_config( new ThisCloudConfig( this ) )
{
}

ThisCloudViewStep::~ThisCloudViewStep() {}

QString
ThisCloudViewStep::prettyName() const
{
    return tr( "THISCLOUD config" );
}

void
ThisCloudViewStep::onLeave()
{
    Calamares::GlobalStorage* gs = Calamares::GlobalStorage::instance();
    if ( gs )
    {
        gs->insert( QStringLiteral( "thiscloudRole" ), m_config->nodeRole() );
        gs->insert( QStringLiteral( "thiscloudClusterName" ), m_config->clusterName() );
        gs->insert( QStringLiteral( "thiscloudNodeIp" ), m_config->nodeIp() );
        gs->insert( QStringLiteral( "thiscloudInterface" ), m_config->interface() );
    }
    Calamares::QmlViewStep::onLeave();
}

bool
ThisCloudViewStep::isNextEnabled() const
{
    // Always allow proceeding; validation is advisory.
    return true;
}

QObject*
ThisCloudViewStep::getConfig()
{
    return m_config;
}

CALAMARES_PLUGIN_FACTORY_DEFINITION( ThisCloudViewStepFactory, registerPlugin< ThisCloudViewStep >(); )